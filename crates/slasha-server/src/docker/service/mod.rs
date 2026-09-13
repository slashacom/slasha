pub mod backup;
pub mod env;
pub mod provision;
pub mod scheduler;
pub mod spec;
pub mod stats;

use std::{
    collections::HashMap,
    io,
    pin::Pin,
    task::{Context, Poll},
};

use bollard::Docker;
use bytes::Bytes;
use chrono::Utc;
pub use env::resolve_service_env;
use futures_util::{Stream, StreamExt, stream::BoxStream};
pub use provision::run_provision_service_workflow;
use slasha_db::{
    app::App,
    logs::{LogPrefix, ResourceKind},
    repos::{
        logs::LogsRepo, node::NodeRepo, s3_storage::S3StorageRepo, service::ServiceRepo,
        service_backup::ServiceBackupRepo,
    },
    service::{
        NewService, NewServiceEnvVar, Service, ServiceKind, ServiceResources, ServiceStatus,
    },
    service_backup::{
        NewServiceBackup, ServiceBackup, ServiceBackupStatus, ServiceBackupTrigger,
        ServiceRestoreStatus,
    },
};
pub use spec::ServiceKindDockerExt;
pub use stats::ServiceStats;
use uuid::Uuid;

use crate::{
    docker::{
        DockerError, DockerResult,
        naming::{service_container_name, service_volume_name},
        service::provision::instance::wait_for_service_health,
        utils::{self, stream_container_logs},
    },
    operations::{self, OperationGuard, ServiceOperation},
    s3,
    state::AppState,
};

/// Stream adapter retaining an [`OperationGuard`] until the stream terminates or drops.
struct GuardedStream<S> {
    stream: S,
    _guard: OperationGuard,
}

impl<S: Stream + Unpin> Stream for GuardedStream<S> {
    type Item = S::Item;

    fn poll_next(mut self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Option<Self::Item>> {
        Pin::new(&mut self.stream).poll_next(cx)
    }
}

#[derive(Clone)]
pub struct ServiceDocker {
    pub state: AppState,
    pub app: App,
    pub docker_client: Docker,
}

impl ServiceDocker {
    /// Creates a new [`ServiceDocker`] handle for an application.
    ///
    /// # Arguments
    ///
    /// * `state` - Application state holding database and runtime handles ([`AppState`]).
    /// * `app` - Target application model ([`App`]).
    ///
    /// # Returns
    ///
    /// A [`DockerResult`] containing a new [`ServiceDocker`] instance.
    pub async fn new(state: AppState, app: App) -> DockerResult<Self> {
        let node = NodeRepo::get(&state.storage.db_pool, &app.node_id).await?;
        let docker_client = state.node_registry.get_client(&node)?;

        Ok(Self {
            state,
            app,
            docker_client,
        })
    }

    fn get_guard(
        &self,
        service_id: &str,
        op: operations::ServiceOperation,
    ) -> Result<operations::OperationGuard, operations::OperationError> {
        self.state
            .runtime
            .operations
            .try_acquire_service(service_id, op)
    }

    /// Provisions a new database service for an application.
    ///
    /// # Arguments
    ///
    /// * `service_kind` - Service type enum ([`ServiceKind`]).
    /// * `name` - Service instance name string.
    /// * `version` - Image version tag string.
    /// * `env_vars` - Map of environment variables for configuration.
    /// * `resources` - Optional resource limit overrides ([`ServiceResources`]).
    ///
    /// # Returns
    ///
    /// A [`DockerResult`] containing the created [`Service`] model.
    pub async fn provision(
        &self,
        service_kind: ServiceKind,
        name: String,
        version: String,
        env_vars: HashMap<String, String>,
        resources: Option<ServiceResources>,
    ) -> DockerResult<Service> {
        if !service_kind
            .supported_versions()
            .contains(&version.as_str())
        {
            return Err(DockerError::Validation(format!(
                "Version {} is not supported for {:?}. Supported versions: {:?}",
                version,
                service_kind,
                service_kind.supported_versions()
            )));
        }

        let mut final_env_vars = service_kind.generate_initial_env_vars();
        final_env_vars.extend(env_vars);

        for (key, val) in &final_env_vars {
            if val.trim().is_empty() {
                return Err(DockerError::Validation(format!(
                    "Environment variable '{}' cannot be empty",
                    key
                )));
            }
        }

        let default_resources = service_kind.default_resources();
        let resources = match resources {
            Some(user_res) => ServiceResources {
                memory_bytes: user_res.memory_bytes.or(default_resources.memory_bytes),
                nano_cpus: user_res.nano_cpus.or(default_resources.nano_cpus),
                pids_limit: user_res.pids_limit.or(default_resources.pids_limit),
                shm_size: user_res.shm_size.or(default_resources.shm_size),
            },
            None => default_resources,
        };

        provision::validate_resources(&self.docker_client, &resources).await?;

        let service_id = Uuid::new_v4().to_string();

        let guard = self.get_guard(&service_id, operations::ServiceOperation::Provisioning)?;

        let new_service = NewService {
            id: service_id.clone(),
            app_id: self.app.id.clone(),
            kind: service_kind,
            name,
            version,
            status: ServiceStatus::Provisioning,
            resources: Some(resources),
            image_digest: None,
        };

        let vars: Vec<NewServiceEnvVar> = final_env_vars
            .into_iter()
            .map(|(key, value)| NewServiceEnvVar {
                service_id: service_id.clone(),
                key,
                value,
            })
            .collect();

        let created_service =
            ServiceRepo::create_with_env_vars(&self.state.storage.db_pool, new_service, vars)
                .await?;

        tokio::spawn({
            let state = self.state.clone();
            let app = self.app.clone();
            let docker_client = self.docker_client.clone();
            let service = created_service.clone();
            let is_redeploy = false;

            async move {
                let _guard = guard;

                if let Err(e) = provision::run_provision_service_workflow(
                    state,
                    app,
                    docker_client,
                    service,
                    is_redeploy,
                )
                .await
                {
                    tracing::error!(error = ?e, "provision workflow failed");
                }
            }
        });

        Ok(created_service)
    }

    /// Gracefully stops a running database service container.
    ///
    /// # Arguments
    ///
    /// * `service_id` - Target service ID string.
    pub async fn stop_service(&self, service_id: &str) -> DockerResult<()> {
        let db_pool = &self.state.storage.db_pool;
        let service = ServiceRepo::find(db_pool, service_id, &self.app.id).await?;

        if service.status != ServiceStatus::Running {
            return Err(DockerError::ServiceNotRunning(service.name));
        }

        let _guard = self.get_guard(&service.id, operations::ServiceOperation::Stopping)?;

        let container_name = service_container_name(&service.id);

        utils::stop_container(&self.docker_client, &container_name, Some(10)).await?;

        ServiceRepo::update_status(db_pool, &service.id, ServiceStatus::Stopped).await?;
        self.state.runtime.log_bus.remove(&service.id);

        Ok(())
    }

    /// Restarts a database service container and waits for health checks to pass.
    ///
    /// # Arguments
    ///
    /// * `service_id` - Target service ID string.
    pub async fn restart_service(&self, service_id: &str) -> DockerResult<()> {
        let db_pool = &self.state.storage.db_pool;
        let service = ServiceRepo::find(db_pool, service_id, &self.app.id).await?;

        if service.status == ServiceStatus::Provisioning {
            return Err(DockerError::PreconditionFailed(format!(
                "Service \"{}\" is currently provisioning",
                service.name
            )));
        }

        let _guard = self.get_guard(&service.id, operations::ServiceOperation::Restarting)?;

        let container_name = service_container_name(&service.id);

        utils::restart_container(&self.docker_client, &container_name).await?;

        let log_writer = self
            .state
            .runtime
            .log_bus
            .writer(ResourceKind::Service, &service.id)
            .app_id(&self.app.id);

        stream_container_logs(
            self.docker_client.clone(),
            log_writer.clone().prefix(LogPrefix::Service),
            container_name.clone(),
        );

        if let Err(e) = wait_for_service_health(
            &self.docker_client,
            &container_name,
            &service.name,
            180,
            &log_writer,
        )
        .await
        {
            ServiceRepo::update_status(db_pool, &service.id, ServiceStatus::Failed).await?;
            return Err(e);
        }

        ServiceRepo::update_status(db_pool, &service.id, ServiceStatus::Running).await?;

        Ok(())
    }

    /// Redeploys a database service. This does not delete the service volume
    ///
    /// # Arguments
    ///
    /// * `service_id` - Target service ID string.
    pub async fn redeploy_service(&self, service_id: &str) -> DockerResult<()> {
        let db_pool = &self.state.storage.db_pool;
        let service = ServiceRepo::find(db_pool, service_id, &self.app.id).await?;

        let guard = self.get_guard(&service.id, operations::ServiceOperation::Provisioning)?;

        ServiceRepo::update_status(db_pool, &service.id, ServiceStatus::Provisioning).await?;

        let container_name = service_container_name(&service.id);

        if let Err(e) = utils::remove_container(&self.docker_client, &container_name).await {
            tracing::warn!(container = %container_name, error = ?e, "Failed to remove service container during redeploy");
        }

        LogsRepo::delete_by_resource_id(&self.state.storage.duckdb_pool, &service.id).await?;

        tokio::spawn({
            let state = self.state.clone();
            let app = self.app.clone();
            let docker_client = self.docker_client.clone();
            let service = service.clone();
            let is_redeploy = true;

            async move {
                let _guard = guard;

                if let Err(e) = provision::run_provision_service_workflow(
                    state,
                    app,
                    docker_client,
                    service,
                    is_redeploy,
                )
                .await
                {
                    tracing::error!(error = ?e, "deployment workflow failed");
                }
            }
        });

        Ok(())
    }

    /// Deletes a stopped or failed database service, removing its container, volume, and records.
    ///
    /// # Arguments
    ///
    /// * `service_id` - Target service ID string.
    pub async fn delete_service(&self, service_id: &str) -> DockerResult<()> {
        let db_pool = &self.state.storage.db_pool;
        let service = ServiceRepo::find(db_pool, service_id, &self.app.id).await?;

        if service.status != ServiceStatus::Stopped && service.status != ServiceStatus::Failed {
            return Err(DockerError::PreconditionFailed(
                "Cannot delete a running or provisioning service. Please stop it first.".into(),
            ));
        }

        let _guard = self.get_guard(&service.id, operations::ServiceOperation::Deleting)?;

        let container_name = service_container_name(&service.id);
        let volume_name = service_volume_name(&service.id);

        if let Err(e) = utils::remove_container(&self.docker_client, &container_name).await {
            tracing::warn!(container = %container_name, error = ?e, "Failed to remove service container");
        }

        if let Err(e) = utils::remove_volume(&self.docker_client, &volume_name).await {
            tracing::warn!(volume = %volume_name, error = ?e, "Failed to remove service volume");
        }

        if let Ok(s3_backups) =
            ServiceBackupRepo::list_for_service_with_s3(db_pool, &service.id).await
        {
            for b in s3_backups {
                if let Some(s3_id) = b.s3_storage_id
                    && let Ok(storage) = S3StorageRepo::find(db_pool, &s3_id).await
                {
                    let key = backup::service_backup_s3_key(&service.id, &b.file_name);
                    let _ = s3::delete_file(&storage, &key).await;
                }
            }
        }

        ServiceRepo::delete(db_pool, &service.id).await?;

        self.state.runtime.log_bus.remove(&service.id);
        let _ = LogsRepo::delete_by_resource_id(&self.state.storage.duckdb_pool, &service.id).await;

        let backup_dir = self.state.storage.get_service_backup_dir(&service.id);
        let _ = tokio::fs::remove_dir_all(&backup_dir).await;

        Ok(())
    }

    /// Triggers a database backup stream for a running service.
    ///
    /// # Arguments
    ///
    /// * `service_id` - Target service ID string.
    ///
    /// # Returns
    ///
    /// A [`DockerResult`] containing a boxed byte stream.
    pub async fn stream_service_backup(
        &self,
        service_id: &str,
    ) -> DockerResult<BoxStream<'static, io::Result<Bytes>>> {
        let db_pool = &self.state.storage.db_pool;
        let service = ServiceRepo::find(db_pool, service_id, &self.app.id).await?;

        if service.status != ServiceStatus::Running {
            return Err(DockerError::ServiceNotRunning(service.name));
        }

        if !service.kind.supports_backups() {
            return Err(DockerError::Validation(
                "Automated backups are not supported for this service kind".into(),
            ));
        }

        let guard = self.get_guard(&service.id, ServiceOperation::BackingUp)?;

        let env_vars = ServiceRepo::get_env_vars(db_pool, &service.id).await?;
        let resolved = resolve_service_env(env_vars, &service)?;

        let stream =
            backup::stream_service_backup(&self.docker_client, &service, &resolved).await?;
        let guarded = GuardedStream {
            stream,
            _guard: guard,
        };
        Ok(guarded.boxed())
    }

    /// Triggers a database backup for a service.
    ///
    /// # Arguments
    ///
    /// * `service_id` - Target service ID string.
    /// * `trigger` - Origin trigger kind ([`ServiceBackupTrigger`]).
    ///
    /// # Returns
    ///
    /// A [`DockerResult`] containing the created [`ServiceBackup`].
    pub async fn run_service_backup(
        &self,
        service_id: &str,
        trigger: ServiceBackupTrigger,
    ) -> DockerResult<ServiceBackup> {
        let db_pool = &self.state.storage.db_pool;
        let service = ServiceRepo::find(db_pool, service_id, &self.app.id).await?;

        if service.status != ServiceStatus::Running {
            return Err(DockerError::ServiceNotRunning(service.name));
        }

        if !service.kind.supports_backups() {
            return Err(DockerError::Validation(
                "Automated backups are not supported for this service kind".into(),
            ));
        }

        let guard = self.get_guard(&service.id, operations::ServiceOperation::BackingUp)?;

        let env_vars = ServiceRepo::get_env_vars(db_pool, &service.id).await?;
        let resolved = resolve_service_env(env_vars, &service)?;

        let config = ServiceBackupRepo::get_config(db_pool, &service.id).await?;
        let keep_local = config.as_ref().map(|c| c.keep_local).unwrap_or(true);
        let s3_storage_id = config.as_ref().and_then(|c| c.s3_storage_id.clone());

        let timestamp = Utc::now().format("%Y%m%d%H%M%S");
        let ext = service.kind.backup_extension();
        let file_name = format!("{}-{}.{}", service.name, timestamp, ext);
        let backup_id = Uuid::new_v4().to_string();

        let new_backup = NewServiceBackup {
            id: backup_id,
            service_id: service.id.clone(),
            s3_storage_id,
            file_name,
            file_size: 0,
            status: ServiceBackupStatus::Running,
            trigger_kind: trigger,
            stored_locally: keep_local,
        };

        let backup = ServiceBackupRepo::create_backup(db_pool, new_backup).await?;

        tokio::spawn({
            let service_docker = self.clone();
            let backup = backup.clone();
            async move {
                let _guard = guard;
                backup::execute_service_backup(service_docker, service, backup, resolved, config)
                    .await;
            }
        });

        Ok(backup)
    }

    /// Restores a database service from a backup snapshot.
    ///
    /// # Arguments
    ///
    /// * `service_id` - Target service ID string.
    /// * `backup_id` - ID string of backup snapshot to restore.
    ///
    /// # Returns
    ///
    /// A [`DockerResult`] indicating restoration success.
    pub async fn run_service_restore(&self, service_id: &str, backup_id: &str) -> DockerResult<()> {
        let db_pool = &self.state.storage.db_pool;
        let service = ServiceRepo::find(db_pool, service_id, &self.app.id).await?;

        if service.status != ServiceStatus::Running {
            return Err(DockerError::ServiceNotRunning(service.name));
        }

        let backup = ServiceBackupRepo::find_backup(db_pool, backup_id).await?;
        if backup.service_id != service.id {
            return Err(DockerError::PreconditionFailed(format!(
                "Service backup \"{}\" not found",
                backup_id
            )));
        }

        if !service.kind.supports_backups() {
            return Err(DockerError::Validation(
                "Automated restore is not supported for this service kind".into(),
            ));
        }

        if backup.status == ServiceBackupStatus::Running {
            return Err(DockerError::PreconditionFailed(
                "Cannot restore while backup is running".into(),
            ));
        }

        if backup.restore_status == ServiceRestoreStatus::Restoring {
            return Err(DockerError::PreconditionFailed(
                "Backup is already currently being restored".into(),
            ));
        }

        if backup.status != ServiceBackupStatus::Succeeded {
            return Err(DockerError::PreconditionFailed(
                "Cannot restore from a failed backup".into(),
            ));
        }

        let guard = self.get_guard(
            &service.id,
            operations::ServiceOperation::Restoring {
                backup_id: backup_id.to_string(),
            },
        )?;

        tokio::spawn({
            let service_docker = self.clone();
            async move {
                let _guard = guard;
                backup::execute_service_restore(service_docker, service, backup).await;
            }
        });

        Ok(())
    }

    /// Deletes a backup snapshot from local disk, S3, and database.
    ///
    /// # Arguments
    ///
    /// * `service_id` - Target service ID string.
    /// * `backup_id` - Target backup ID string.
    ///
    /// # Returns
    ///
    /// A [`DockerResult`] indicating deletion success.
    pub async fn delete_service_backup(
        &self,
        service_id: &str,
        backup_id: &str,
    ) -> DockerResult<()> {
        let db_pool = &self.state.storage.db_pool;
        let service = ServiceRepo::find(db_pool, service_id, &self.app.id).await?;

        let backup = ServiceBackupRepo::find_backup(db_pool, backup_id).await?;
        if backup.service_id != service.id {
            return Err(DockerError::PreconditionFailed(format!(
                "Service backup \"{}\" not found",
                backup_id
            )));
        }

        if backup.status == ServiceBackupStatus::Running {
            return Err(DockerError::PreconditionFailed(
                "Cannot delete backup while it is running".into(),
            ));
        }

        if backup.restore_status == ServiceRestoreStatus::Restoring {
            return Err(DockerError::PreconditionFailed(
                "Cannot delete backup while it is being restored".into(),
            ));
        }

        let service_key = operations::ResourceKey::Service(service.id.as_str().into());
        self.state.runtime.operations.ensure_idle(&service_key)?;

        backup::delete_service_backup_files(&self.state, &service, &backup).await?;

        ServiceBackupRepo::delete_backup(db_pool, backup_id).await?;

        Ok(())
    }

    /// Fetches resource usage statistics for a database service.
    ///
    /// # Arguments
    ///
    /// * `service_id` - Target service ID string.
    ///
    /// # Returns
    ///
    /// A [`DockerResult`] containing a [`ServiceStats`] struct.
    pub async fn get_stats(&self, service_id: &str) -> DockerResult<ServiceStats> {
        let db_pool = &self.state.storage.db_pool;
        let service = ServiceRepo::find(db_pool, service_id, &self.app.id).await?;

        stats::get_service_stats(&self.docker_client, &service)
            .await
            .ok_or_else(|| DockerError::Other(anyhow::anyhow!("failed to fetch service stats")))
    }
}
