pub mod app_move;
pub mod deploy;
pub mod env;
pub mod image;
pub mod litestream;
pub mod network;
pub mod parser;
pub mod process;
pub mod purge;
pub mod scale;
pub mod volume;

use bollard::Docker;
use chrono::Utc;
pub use env::resolve_app_env;
use slasha_db::{
    app::{App, AppSource},
    deployment::{Deployment, DeploymentStatus, NewDeployment},
    models::{
        app_scale::{ProcessContainer, ProcessType},
        node::NodeStatus,
    },
    repos::{
        app::AppRepo, app_backup::AppBackupRepo, deployment::DeploymentRepo, logs::LogsRepo,
        node::NodeRepo,
    },
};
use uuid::Uuid;
pub use volume::AppVolume;

use crate::{
    connections,
    docker::{
        DockerError, DockerResult,
        app::{purge::purge_app_from_node, scale::scale_deployment_process},
    },
    operations::{self, ActiveOperation, AppOperation, ResourceKey},
    state::AppState,
};

#[derive(Clone)]
pub struct AppDocker {
    pub state: AppState,
    pub app: App,
    pub docker_client: Docker,
}

impl AppDocker {
    /// Creates a new [`AppDocker`] handle for an application.
    ///
    /// # Arguments
    ///
    /// * `state` - Application state holding database and runtime handles ([`AppState`]).
    /// * `app` - Target application model ([`App`]).
    ///
    /// # Returns
    ///
    /// A [`DockerResult`] containing a new [`AppDocker`] instance.
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
        op: AppOperation,
    ) -> Result<operations::OperationGuard, operations::OperationError> {
        self.state
            .runtime
            .operations
            .try_acquire_app(&self.app.id, op)
    }

    fn get_deployment_guard(
        &self,
        deployment_id: impl Into<String>,
    ) -> Result<
        (
            operations::OperationGuard,
            tokio_util::sync::CancellationToken,
        ),
        operations::OperationError,
    > {
        let cancel_token = tokio_util::sync::CancellationToken::new();
        let guard = self.get_guard(operations::AppOperation::Deploying {
            deployment_id: deployment_id.into(),
            cancel_token: cancel_token.clone(),
        })?;

        Ok((guard, cancel_token))
    }

    /// Triggers a new deployment for an application.
    ///
    /// # Arguments
    ///
    /// * `commit_sha` - Optional Git commit SHA to deploy instead of default branch HEAD.
    ///
    /// # Returns
    ///
    /// A [`DockerResult`] containing the created [`Deployment`] model.
    pub async fn deploy(&self, commit_sha: Option<String>) -> DockerResult<Deployment> {
        if self.app.source != AppSource::Local {
            connections::sync_external_app(
                self.state.github_client().await.as_ref(),
                &self.state.storage,
                &self.state.runtime,
                &self.app,
            )
            .await?;
        }

        let deployment_id = Uuid::new_v4().to_string();
        let (guard, cancel_token) = self.get_deployment_guard(&deployment_id)?;

        let (commit_sha, commit_message) = match commit_sha {
            Some(sha) => {
                let msg = deploy::context::resolve_commit_message(&self.app.repo_path, &sha)?;
                (sha, msg)
            }
            None => {
                deploy::context::resolve_head_commit(&self.app.repo_path, &self.app.default_branch)?
            }
        };

        let deployment = NewDeployment {
            id: deployment_id,
            app_id: self.app.id.clone(),
            commit_sha,
            commit_message,
            status: DeploymentStatus::Pending,
            node_id: self.app.node_id.clone(),
        };

        let deployment = DeploymentRepo::create(&self.state.storage.db_pool, deployment).await?;

        tokio::spawn({
            let state = self.state.clone();
            let app = self.app.clone();
            let docker_client = self.docker_client.clone();
            let deployment = deployment.clone();

            async move {
                let _guard = guard;

                if let Err(e) = deploy::run_deployment_workflow(
                    state,
                    app,
                    docker_client,
                    deployment,
                    None,
                    cancel_token,
                )
                .await
                {
                    tracing::error!(error = ?e, "deployment workflow failed");
                }
            }
        });

        Ok(deployment)
    }

    /// Redeploys an existing deployment.
    ///
    /// # Arguments
    ///
    /// * `deployment_id` - Deployment ID string to redeploy.
    ///
    /// # Returns
    ///
    /// A [`DockerResult`] containing the reset [`Deployment`] model.
    pub async fn redeploy(&self, deployment_id: &str) -> DockerResult<Deployment> {
        let (guard, cancel_token) = self.get_deployment_guard(deployment_id)?;

        let now = Utc::now().naive_utc();
        let deployment =
            DeploymentRepo::reset_to_pending(&self.state.storage.db_pool, deployment_id, now)
                .await?;

        tokio::spawn({
            let state = self.state.clone();
            let app = self.app.clone();
            let docker_client = self.docker_client.clone();
            let deployment = deployment.clone();

            async move {
                let _guard = guard;

                if let Err(e) = deploy::run_deployment_workflow(
                    state,
                    app,
                    docker_client,
                    deployment,
                    None,
                    cancel_token,
                )
                .await
                {
                    tracing::error!(error = ?e, "redeployment workflow failed");
                }
            }
        });

        Ok(deployment)
    }

    /// Triggers a rollback to a previous deployment.
    ///
    /// Reuses the deployment's retained Docker image if available;
    /// otherwise rebuilds it from the deployment's commit.
    ///
    /// # Arguments
    ///
    /// * `deployment_id` - ID of the deployment to roll back to.
    ///
    /// # Returns
    ///
    /// A [`DockerResult`] containing the new [`Deployment`].
    pub async fn rollback_to_deployment(&self, deployment_id: &str) -> DockerResult<Deployment> {
        let source_deployment =
            DeploymentRepo::find(&self.state.storage.db_pool, deployment_id, &self.app.id).await?;

        let new_deployment_id = Uuid::new_v4().to_string();
        let (guard, cancel_token) = self.get_deployment_guard(&new_deployment_id)?;

        let source_image = image::find_deployment_image(
            &self.docker_client,
            &self.app.slug,
            &source_deployment.id,
        )
        .await
        .ok();

        let deployment = NewDeployment {
            id: new_deployment_id,
            app_id: self.app.id.clone(),
            commit_sha: source_deployment.commit_sha,
            commit_message: source_deployment.commit_message,
            status: DeploymentStatus::Pending,
            node_id: source_deployment.node_id,
        };

        let deployment = DeploymentRepo::create(&self.state.storage.db_pool, deployment).await?;

        tokio::spawn({
            let state = self.state.clone();
            let app = self.app.clone();
            let docker_client = self.docker_client.clone();
            let deployment = deployment.clone();

            async move {
                let _guard = guard;

                if let Err(e) = deploy::run_deployment_workflow(
                    state,
                    app,
                    docker_client,
                    deployment,
                    source_image,
                    cancel_token,
                )
                .await
                {
                    tracing::error!(error = ?e, "rollback workflow failed");
                }
            }
        });

        Ok(deployment)
    }

    /// Scales active process containers for a deployment to a target replica count.
    ///
    /// # Arguments
    ///
    /// * `deployment_id` - Target deployment ID string.
    /// * `process_type` - Process type enum ([`ProcessType`]).
    /// * `count` - Target replica count.
    pub async fn scale(
        &self,
        deployment_id: &str,
        process_type: ProcessType,
        count: u32,
    ) -> DockerResult<()> {
        let db_pool = &self.state.storage.db_pool;

        if process_type == ProcessType::Web && count > 1 {
            let backups_on = AppBackupRepo::get(db_pool, &self.app.id)
                .await?
                .is_some_and(|b| b.enabled);
            if backups_on {
                return Err(DockerError::EnvResolveFailed(
                    "Backups require a single web instance (Litestream must be the only writer). Disable backups to scale web beyond 1.".to_string(),
                ));
            }
        }

        let deployment = DeploymentRepo::find(db_pool, deployment_id, &self.app.id).await?;

        if deployment.status != DeploymentStatus::Running {
            return Err(DockerError::EnvResolveFailed(
                "Scaling is only allowed for running deployments".to_string(),
            ));
        }

        let guard = self.get_guard(AppOperation::Scaling)?;

        tokio::spawn({
            let state = self.state.clone();
            let app = self.app.clone();
            let deployment = deployment.clone();

            async move {
                let _guard = guard;

                if let Err(e) =
                    scale_deployment_process(&state, &app, &deployment, process_type, count).await
                {
                    tracing::error!(
                        app_slug = %app.slug,
                        deployment_id = %deployment.id,
                        error = ?e,
                        "failed to scale deployment process"
                    );
                }
            }
        });

        Ok(())
    }

    /// Cancels an in-flight deployment workflow.
    ///
    /// # Arguments
    ///
    /// * `deployment_id` - Deployment ID string to cancel.
    pub async fn cancel_deployment(&self, deployment_id: &str) -> DockerResult<()> {
        let deployment =
            DeploymentRepo::find(&self.state.storage.db_pool, deployment_id, &self.app.id).await?;

        if !matches!(
            deployment.status,
            DeploymentStatus::Building | DeploymentStatus::Pending
        ) {
            return Err(crate::docker::DockerError::EnvResolveFailed(format!(
                "Deployment is in state '{}' and cannot be cancelled",
                deployment.status
            )));
        }

        let key = ResourceKey::app(&self.app.id);

        if let Some(op) = self.state.runtime.operations.get_operation(&key)
            && let ActiveOperation::App(AppOperation::Deploying {
                deployment_id: active_id,
                cancel_token,
            }) = op.value()
            && active_id == deployment_id
        {
            cancel_token.cancel();
        }

        Ok(())
    }

    /// Gracefully stops active process containers associated with a deployment.
    ///
    /// # Arguments
    ///
    /// * `deployment_id` - Target deployment ID string.
    pub async fn stop_deployment(&self, deployment_id: &str) -> DockerResult<()> {
        let deployment =
            DeploymentRepo::find(&self.state.storage.db_pool, deployment_id, &self.app.id).await?;

        if deployment.status != DeploymentStatus::Running {
            return Err(crate::docker::DockerError::EnvResolveFailed(format!(
                "Deployment is in state '{}' and cannot be stopped",
                deployment.status
            )));
        }

        let _guard = self.get_guard(AppOperation::Stopping)?;

        process::stop_deployment_processes(&self.docker_client, &deployment).await?;

        self.state.runtime.log_bus.remove(&deployment.id);

        DeploymentRepo::update_status(
            &self.state.storage.db_pool,
            &deployment.id,
            DeploymentStatus::Stopped,
        )
        .await?;

        self.state.runtime.proxy_sync_trigger.notify_one();

        Ok(())
    }

    /// Restarts active process containers for a deployment.
    ///
    /// # Arguments
    ///
    /// * `deployment_id` - Target deployment ID string.
    pub async fn restart_deployment(&self, deployment_id: &str) -> DockerResult<()> {
        let deployment =
            DeploymentRepo::find(&self.state.storage.db_pool, deployment_id, &self.app.id).await?;

        let _guard = self.get_guard(AppOperation::Restarting)?;

        process::restart_deployment_processes(
            &self.docker_client,
            &self.state.runtime,
            &self.app,
            &deployment,
        )
        .await?;

        DeploymentRepo::update_status(
            &self.state.storage.db_pool,
            &deployment.id,
            DeploymentStatus::Running,
        )
        .await?;

        self.state.runtime.proxy_sync_trigger.notify_one();

        Ok(())
    }

    /// Deletes a deployment, removing process containers, cached image artifacts, and database records.
    ///
    /// # Arguments
    ///
    /// * `deployment_id` - Target deployment ID string.
    pub async fn delete_deployment(&self, deployment_id: &str) -> DockerResult<()> {
        let deployment =
            DeploymentRepo::find(&self.state.storage.db_pool, deployment_id, &self.app.id).await?;

        let _guard = self.get_guard(AppOperation::Deleting)?;

        process::remove_deployment_processes(&self.docker_client, &deployment).await?;

        self.state.runtime.proxy_sync_trigger.notify_one();

        image::remove_deployment_image(&self.docker_client, &self.app.slug, &deployment.id).await?;

        self.state.runtime.log_bus.remove(&deployment.id);
        LogsRepo::delete_by_resource_id(&self.state.storage.duckdb_pool, &deployment.id).await?;

        DeploymentRepo::delete(&self.state.storage.db_pool, &deployment.id, &self.app.id).await?;

        Ok(())
    }

    /// Lists active process containers associated with a deployment.
    ///
    /// # Arguments
    ///
    /// * `deployment_id` - Target deployment ID string.
    ///
    /// # Returns
    ///
    /// A [`DockerResult`] containing a vector of [`ProcessContainer`]s.
    pub async fn list_processes(&self, deployment_id: &str) -> DockerResult<Vec<ProcessContainer>> {
        process::list_deployment_processes(&self.docker_client, deployment_id).await
    }

    /// Migrates an application, data volumes, networks, and services to a new target host node.
    ///
    /// # Arguments
    ///
    /// * `target_node_id` - Destination node ID string.
    pub async fn move_to_node(&self, target_node_id: &str) -> DockerResult<()> {
        if self.app.node_id == target_node_id {
            return Err(crate::docker::DockerError::EnvResolveFailed(
                "App is already on the target node".to_string(),
            ));
        }

        let target_node = NodeRepo::get(&self.state.storage.db_pool, target_node_id).await?;
        if target_node.status != NodeStatus::Ready {
            return Err(crate::docker::DockerError::EnvResolveFailed(
                "Target node is not ready".to_string(),
            ));
        }

        let guard = self.get_guard(AppOperation::Migrating)?;

        let target_docker_client = self
            .state
            .clients
            .docker_registry
            .get_client(&target_node)?;

        tokio::spawn({
            let _guard = guard;
            let state = self.state.clone();
            let app = self.app.clone();
            let source_docker_client = self.docker_client.clone();

            async move {
                if let Err(e) = app_move::run_move_app_workflow(
                    state,
                    app,
                    source_docker_client,
                    target_docker_client,
                    target_node,
                )
                .await
                {
                    tracing::error!(error = ?e, "move to node workflow failed");
                }
            }
        });

        Ok(())
    }

    /// Purges all application containers, data volumes, networks, images, and source files from a node.
    pub async fn purge_from_node(&self) -> DockerResult<()> {
        let guard = self.get_guard(AppOperation::Purging)?;

        tokio::spawn({
            let state = self.state.clone();
            let app = self.app.clone();
            let docker_client = self.docker_client.clone();

            async move {
                let _guard = guard;

                if let Err(e) = purge_app_from_node(&app, &docker_client, &state.storage).await {
                    tracing::error!(app_id = %app.id, error = ?e, "failed to purge app containers/volumes from node");
                }

                state.runtime.proxy_sync_trigger.notify_one();
                let _ = LogsRepo::delete_by_app_id(&state.storage.duckdb_pool, &app.id).await;

                let repo_path = std::path::PathBuf::from(&app.repo_path);
                if repo_path.exists()
                    && let Err(e) = tokio::fs::remove_dir_all(&repo_path).await
                {
                    tracing::error!(app_slug = %app.slug, error = ?e, "failed to remove repo directory");
                }

                let _ = AppRepo::delete(&state.storage.db_pool, &app.id).await;
            }
        });

        Ok(())
    }

    /// Lists all persistent volume mounts associated with an application.
    ///
    /// # Returns
    ///
    /// A [`DockerResult`] containing a vector of [`AppVolume`]s.
    pub async fn list_volumes(&self) -> DockerResult<Vec<AppVolume>> {
        volume::list_app_volumes(&self.docker_client, &self.app).await
    }
}
