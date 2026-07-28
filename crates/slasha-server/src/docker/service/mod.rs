pub mod env;
pub mod provision;
pub mod spec;
pub mod stats;

use std::collections::HashMap;

use bollard::{
    Docker,
    exec::{CreateExecOptions, StartExecOptions, StartExecResults},
};
pub use env::resolve_service_env;
use futures_util::StreamExt;
pub use provision::run_provision_service_workflow;
use slasha_db::{
    app::App,
    repos::{node::NodeRepo, service::ServiceRepo},
    service::{
        NewService, NewServiceEnvVar, Service, ServiceKind, ServiceResources, ServiceStatus,
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
        utils,
    },
    logs::{LogKey, stream_container_logs},
    operations,
    state::AppState,
};

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
        let docker_client = state.clients.docker_registry.get_client(&node)?;

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

        self.validate_resources(&resources).await?;

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
                    tracing::error!(error = ?e, "deployment workflow failed");
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
        let log_key = LogKey::Service {
            app_slug: self.app.slug.clone(),
            service_name: service.name.clone(),
        };

        utils::stop_container(&self.docker_client, &container_name, Some(10)).await?;

        ServiceRepo::update_status(db_pool, &service.id, ServiceStatus::Stopped).await?;
        self.state.runtime.log_manager.remove(&log_key);

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
            return Err(DockerError::Validation(format!(
                "Service {} is currently provisioning",
                service.name
            )));
        }

        let _guard = self.get_guard(&service.id, operations::ServiceOperation::Restarting)?;

        let container_name = service_container_name(&service.id);
        let log_key = LogKey::Service {
            app_slug: self.app.slug.clone(),
            service_name: service.name.clone(),
        };

        utils::restart_container(&self.docker_client, &container_name).await?;

        let log = self.state.runtime.log_manager.get_logger(&log_key).await?;
        stream_container_logs(
            self.docker_client.clone(),
            log.clone(),
            container_name.clone(),
            None,
        );

        if let Err(e) = wait_for_service_health(
            &self.docker_client,
            &container_name,
            &service.name,
            180,
            Some(&log),
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
        let log_key = LogKey::Service {
            app_slug: self.app.slug.clone(),
            service_name: service.name.clone(),
        };

        if let Err(e) = utils::remove_container(&self.docker_client, &container_name).await {
            tracing::warn!(container = %container_name, error = ?e, "Failed to remove service container during redeploy");
        }

        self.state.runtime.log_manager.remove(&log_key);

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
            return Err(DockerError::Validation(
                "Cannot delete a running or provisioning service. Please stop it first.".into(),
            ));
        }

        let _guard = self.get_guard(&service.id, operations::ServiceOperation::Deleting)?;

        let container_name = service_container_name(&service.id);
        let volume_name = service_volume_name(&service.id);
        let log_key = LogKey::Service {
            app_slug: self.app.slug.clone(),
            service_name: service.name.clone(),
        };

        if let Err(e) = utils::remove_container(&self.docker_client, &container_name).await {
            tracing::warn!(container = %container_name, error = ?e, "Failed to remove service container");
        }

        if let Err(e) = utils::remove_volume(&self.docker_client, &volume_name).await {
            tracing::warn!(volume = %volume_name, error = ?e, "Failed to remove service volume");
        }

        ServiceRepo::delete(db_pool, &service.id).await?;
        self.state.runtime.log_manager.remove(&log_key);

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
    pub async fn backup_service(
        &self,
        service_id: &str,
    ) -> DockerResult<futures_util::stream::BoxStream<'static, Result<bytes::Bytes, std::io::Error>>>
    {
        let db_pool = &self.state.storage.db_pool;
        let service = ServiceRepo::find(db_pool, service_id, &self.app.id).await?;

        if service.status != ServiceStatus::Running {
            return Err(DockerError::ServiceNotRunning(service.name));
        }

        let _guard = self.get_guard(&service.id, operations::ServiceOperation::BackingUp)?;

        let env_vars = ServiceRepo::get_env_vars(db_pool, &service.id).await?;
        let resolved = resolve_service_env(env_vars, &service)?;

        let cmd = service.kind.backup_cmd(&resolved);
        let container_name = service_container_name(&service.id);

        let exec_id = self
            .docker_client
            .create_exec(
                &container_name,
                CreateExecOptions {
                    attach_stdout: Some(true),
                    attach_stderr: Some(false),
                    cmd: Some(cmd),
                    ..Default::default()
                },
            )
            .await?;

        let output_stream = match self
            .docker_client
            .start_exec(&exec_id.id, None::<StartExecOptions>)
            .await?
        {
            StartExecResults::Attached { output, .. } => output,
            StartExecResults::Detached => {
                return Err(DockerError::Other(anyhow::anyhow!(
                    "exec returned detached"
                )));
            }
        };

        let byte_stream = output_stream.filter_map(|item| async move {
            match item {
                Ok(bollard::container::LogOutput::StdOut { message }) => Some(Ok(message)),
                _ => None,
            }
        });

        Ok(byte_stream.boxed())
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

    /// Validates requested service resource limits against host node capacity caps.
    ///
    /// # Arguments
    ///
    /// * `resources` - Requested resource limits ([`ServiceResources`]).
    async fn validate_resources(&self, resources: &ServiceResources) -> DockerResult<()> {
        const MIN_MEMORY_BYTES: i64 = 64 * 1024 * 1024;
        const MIN_NANO_CPUS: i64 = 100_000_000;
        const MIN_SHM_BYTES: i64 = 64 * 1024 * 1024;
        const MIN_PIDS_LIMIT: i64 = 64;

        if let Some(mem) = resources.memory_bytes
            && mem < MIN_MEMORY_BYTES
        {
            return Err(DockerError::Validation(format!(
                "memory must be at least {} MB",
                MIN_MEMORY_BYTES / (1024 * 1024)
            )));
        }
        if let Some(nc) = resources.nano_cpus
            && nc < MIN_NANO_CPUS
        {
            return Err(DockerError::Validation(
                "CPU must be at least 0.1 cores".into(),
            ));
        }
        if let Some(shm) = resources.shm_size
            && shm < MIN_SHM_BYTES
        {
            return Err(DockerError::Validation(format!(
                "shared memory must be at least {} MB",
                MIN_SHM_BYTES / (1024 * 1024)
            )));
        }
        if let Some(pids) = resources.pids_limit
            && pids < MIN_PIDS_LIMIT
        {
            return Err(DockerError::Validation(format!(
                "PID limit must be at least {}",
                MIN_PIDS_LIMIT
            )));
        }

        let info = self.docker_client.info().await?;

        if let Some(host_mem) = info.mem_total
            && let Some(mem) = resources.memory_bytes
        {
            let max_allowed_mem = (host_mem as f64 * 0.80) as i64;
            if mem > max_allowed_mem {
                return Err(DockerError::Validation(format!(
                    "Requested memory ({} MB) exceeds 80% host capacity cap ({} MB of {} MB total host RAM)",
                    mem / (1024 * 1024),
                    max_allowed_mem / (1024 * 1024),
                    host_mem / (1024 * 1024)
                )));
            }
        }
        if let Some(host_cpus) = info.ncpu
            && let Some(nc) = resources.nano_cpus
        {
            let host_nano = host_cpus.saturating_mul(1_000_000_000);
            if nc > host_nano {
                return Err(DockerError::Validation(format!(
                    "CPU ({:.2} cores) exceeds host capacity ({} cores)",
                    nc as f64 / 1_000_000_000.0,
                    host_cpus
                )));
            }
        }
        if let Some(host_mem) = info.mem_total
            && let Some(shm) = resources.shm_size
            && shm > host_mem
        {
            return Err(DockerError::Validation(format!(
                "shared memory ({} MB) exceeds host capacity ({} MB)",
                shm / (1024 * 1024),
                host_mem / (1024 * 1024)
            )));
        }

        Ok(())
    }
}
