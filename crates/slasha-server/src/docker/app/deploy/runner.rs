use bollard::{Docker, query_parameters::RemoveImageOptions};
use slasha_db::{
    app::App,
    deployment::{Deployment, DeploymentStatus},
    models::app_scale::{AppScale, ProcessType},
    node::LOCAL_NODE_ID,
    repos::{app_backup::AppBackupRepo, app_scale::AppScaleRepo, deployment::DeploymentRepo},
};

use super::{
    build::{build_docker, build_railpack},
    context::DeploymentContext,
    readiness::{ReadinessConfig, ReadinessOutcome, wait_for_web_ready},
};
use crate::{
    docker::{
        DockerError, DockerResult,
        app::{
            deploy::{
                create::{CreateContainerContext, create_process_container},
                release::run_release_container,
            },
            image::{image_tag, prune_app_images, tag_deployment_image},
            litestream,
            parser::{BuildStrategy, Procfile},
            process::{
                remove_deployment_processes, start_process_container, stop_deployment_processes,
            },
        },
        naming::process_container_name,
        utils,
        workflow::runner::WorkflowContext,
    },
    logs::LogHandle,
    state::AppState,
};

struct ProcessTarget {
    process_type: ProcessType,
    command: Option<String>,
    count: u32,
}

/// Maps Procfile commands and scale configurations into process targets.
///
/// # Arguments
///
/// * `procfile` - Parsed Procfile definitions ([`Procfile`]).
/// * `scale_configs` - Database scale configurations ([`AppScale`]).
///
/// # Returns
///
/// A vector of process targets ([`ProcessTarget`]).
fn resolve_process_targets(
    procfile: &Option<Procfile>,
    scale_configs: &[AppScale],
) -> Vec<ProcessTarget> {
    let mut targets = Vec::new();

    if let Some(pf) = procfile {
        for (pt, cmd) in &pf.commands {
            if *pt == ProcessType::Release {
                continue;
            }

            let count = scale_configs
                .iter()
                .find(|s| s.process_type == *pt)
                .map(|s| s.desired as u32)
                .unwrap_or(1);

            targets.push(ProcessTarget {
                process_type: *pt,
                command: Some(cmd.clone()),
                count,
            });
        }
    } else {
        targets.push(ProcessTarget {
            process_type: ProcessType::Web,
            command: None,
            count: 1,
        });
    }

    targets
}

pub struct DeploymentRunner<'a> {
    pub state: &'a AppState,
    pub app: &'a App,
    pub docker_client: &'a Docker,
    pub deployment: &'a Deployment,
    pub wf: &'a WorkflowContext<'a>,
    pub log: &'a LogHandle,
    pub context: &'a DeploymentContext,
}

impl<'a> DeploymentRunner<'a> {
    /// Executes the application deployment pipeline sequentially.
    ///
    /// # Arguments
    ///
    /// * `source_image` - Optional source image tag to reuse instead of building.
    pub async fn execute(&self, source_image: Option<&str>) -> DockerResult<()> {
        self.build_and_tag_image(source_image).await?;

        if let Some(procfile) = &self.context.procfile
            && let Some(cmd) = procfile.get_process_command(&ProcessType::Release)
        {
            self.wf
                .step(
                    "Running release container",
                    run_release_container(
                        self.docker_client,
                        self.log,
                        self.app,
                        self.deployment,
                        cmd,
                        &self.context.env_map,
                    ),
                    {
                        let docker_client = self.docker_client.clone();
                        let release_container_name = process_container_name(
                            &self.app.id,
                            &self.deployment.id,
                            &ProcessType::Release,
                            0,
                        );
                        async move {
                            let _ =
                                utils::remove_container(&docker_client, &release_container_name)
                                    .await;
                        }
                    },
                )
                .await?;
        }

        let backup = AppBackupRepo::get(&self.state.storage.db_pool, &self.app.id)
            .await
            .ok()
            .flatten();

        // Stateful apps stop old containers first to prevent volume write conflicts
        let is_stateful =
            !self.context.volume_paths.is_empty() || backup.as_ref().is_some_and(|b| b.enabled);

        if is_stateful {
            let previous_deployments = self.get_previous_deployments().await?;
            if !previous_deployments.is_empty() {
                let log = self.log.clone();
                self.wf
                    .step(
                        "Stopping previous deployment processes for stateful application",
                        self.stop_previous_deployments(),
                        {
                            let docker_client = self.docker_client.clone();
                            let app = self.app.clone();
                            async move {
                                for prev in &previous_deployments {
                                    let _ = start_process_container(
                                        &docker_client,
                                        &log,
                                        &app,
                                        prev,
                                        ProcessType::Web,
                                        0,
                                    )
                                    .await;
                                }
                            }
                        },
                    )
                    .await?;
            }
        }

        let created_containers = self.create_deployment_containers().await?;

        for (pt, i) in &created_containers {
            start_process_container(
                self.docker_client,
                self.log,
                self.app,
                self.deployment,
                *pt,
                *i,
            )
            .await?;
        }

        self.enforce_readiness(&created_containers).await?;

        if backup
            .as_ref()
            .is_some_and(|b| b.enabled && b.restore_pending)
        {
            self.log
                .send("Restored SQLite database from backup replica".to_string())
                .await?;
            if let Err(e) =
                AppBackupRepo::set_restore_pending(&self.state.storage.db_pool, &self.app.id, false)
                    .await
            {
                tracing::warn!(app_id = %self.app.id, error = ?e, "Failed to clear restore_pending");
            }
        }

        self.promote_active().await?;

        Ok(())
    }

    /// Builds the application Docker image or tags a retained source image.
    ///
    /// # Arguments
    ///
    /// * `source_image` - Optional source image tag to reuse.
    async fn build_and_tag_image(&self, source_image: Option<&str>) -> DockerResult<()> {
        let db_pool = &self.state.storage.db_pool;

        DeploymentRepo::update_status(db_pool, &self.deployment.id, DeploymentStatus::Building)
            .await?;

        let tag = image_tag(&self.app.slug, &self.deployment.id);

        self.wf
            .step(
                "Building application Docker image",
                async {
                    if let Some(source_image) = source_image {
                        self.log
                            .send(format!("Reusing retained image {}", source_image))
                            .await?;

                        tag_deployment_image(
                            self.docker_client,
                            source_image,
                            &self.app.slug,
                            &self.deployment.id,
                        )
                        .await?;
                    } else {
                        let build_label = self.context.strategy.to_string();
                        self.log
                            .send(format!(
                                "Building image slasha/{}:{} ({})",
                                self.app.slug, self.deployment.id, build_label
                            ))
                            .await?;

                        match &self.context.strategy {
                            BuildStrategy::Dockerfile { .. } => {
                                build_docker(
                                    self.docker_client,
                                    self.log,
                                    self.app,
                                    self.deployment,
                                )
                                .await?
                            }
                            BuildStrategy::Railpack => {
                                build_railpack(
                                    self.docker_client,
                                    self.log,
                                    self.app,
                                    self.deployment,
                                )
                                .await?
                            }
                        }
                    }
                    Ok::<(), DockerError>(())
                },
                {
                    let docker_client = self.docker_client.clone();
                    let tag = tag.clone();
                    async move {
                        let _ = docker_client
                            .remove_image(
                                &tag,
                                Some(RemoveImageOptions {
                                    force: true,
                                    ..Default::default()
                                }),
                                None,
                            )
                            .await;
                    }
                },
            )
            .await?;

        Ok(())
    }

    /// Creates process containers for the new deployment.
    ///
    /// # Returns
    ///
    /// A [`DockerResult`] containing a vector of process type and instance index tuples `(ProcessType, u32)`.
    async fn create_deployment_containers(&self) -> DockerResult<Vec<(ProcessType, u32)>> {
        let db_pool = &self.state.storage.db_pool;
        let scale_configs = AppScaleRepo::list_for_app(db_pool, &self.app.id).await?;
        let targets = resolve_process_targets(&self.context.procfile, &scale_configs);

        let backup = AppBackupRepo::get(db_pool, &self.app.id)
            .await
            .ok()
            .flatten();

        let litestream_volume = if backup.as_ref().is_some_and(|b| b.enabled) {
            match litestream::ensure_litestream_volume(self.docker_client).await {
                Ok(volume) => Some(volume),
                Err(e) => {
                    let _ = self
                        .log
                        .send(format!(
                            "Warning: could not prepare litestream binary, skipping backups: {e}"
                        ))
                        .await;
                    None
                }
            }
        } else {
            None
        };

        // Clean up any stale containers for this deployment prior to creation.
        // Only activates when redeploying, it has no effect on new deployments.
        remove_deployment_processes(self.docker_client, self.deployment).await?;

        let mut created_containers = Vec::new();

        for target in targets {
            for i in 0..target.count {
                let container_name = process_container_name(
                    &self.app.id,
                    &self.deployment.id,
                    &target.process_type,
                    i,
                );

                self.wf
                    .step(
                        format!(
                            "Creating {} process container (instance {})",
                            target.process_type, i
                        ),
                        create_process_container(
                            self.docker_client,
                            self.app,
                            self.deployment,
                            CreateContainerContext {
                                process_type: target.process_type,
                                instance_index: i,
                                container_port: Some(self.context.container_port),
                                cmd: target.command.as_deref(),
                                env_map: &self.context.env_map,
                                volume_paths: &self.context.volume_paths,
                                backup: backup.as_ref(),
                                litestream_volume: litestream_volume.as_deref(),
                            },
                        ),
                        {
                            let docker_client = self.docker_client.clone();
                            let container_name = container_name.clone();
                            async move {
                                let _ =
                                    utils::remove_container(&docker_client, &container_name).await;
                            }
                        },
                    )
                    .await?;

                created_containers.push((target.process_type, i));
            }
        }

        Ok(created_containers)
    }

    /// Gates the release by probing web process containers until healthy.
    ///
    /// # Arguments
    ///
    /// * `created_containers` - Created process container tuples `(ProcessType, u32)`.
    async fn enforce_readiness(
        &self,
        created_containers: &[(ProcessType, u32)],
    ) -> DockerResult<()> {
        let web_containers: Vec<String> = created_containers
            .iter()
            .filter(|(pt, _)| *pt == ProcessType::Web)
            .map(|(pt, i)| process_container_name(&self.app.id, &self.deployment.id, pt, *i))
            .collect();

        if !web_containers.is_empty() {
            let is_local = self.app.node_id == LOCAL_NODE_ID;
            let config = ReadinessConfig::from_env_map(&self.context.env_map);

            self.log
                .send(format!(
                    "Waiting for web process to respond on GET {} (timeout: {}s)",
                    config.path,
                    config.timeout.as_secs()
                ))
                .await?;

            for container_name in &web_containers {
                match wait_for_web_ready(
                    self.docker_client,
                    container_name,
                    self.context.container_port,
                    &config,
                    is_local,
                )
                .await
                {
                    ReadinessOutcome::Ready { elapsed } => {
                        self.log
                            .send(format!(
                                "{} became ready in {:.1}s",
                                container_name,
                                elapsed.as_secs_f64()
                            ))
                            .await?;
                    }
                    ReadinessOutcome::Unreachable => {
                        return Err(DockerError::AppNotReady(format!(
                            "Container {} is unreachable on the network",
                            container_name
                        )));
                    }
                    ReadinessOutcome::NotReady { reason } => {
                        return Err(DockerError::AppNotReady(reason));
                    }
                }
            }
        }

        Ok(())
    }

    /// Fetches active deployments for the application prior to promotion.
    ///
    /// # Returns
    ///
    /// A [`DockerResult`] containing a vector of active [`Deployment`] models.
    async fn get_previous_deployments(&self) -> DockerResult<Vec<Deployment>> {
        let previous_deployments: Vec<Deployment> =
            DeploymentRepo::list_active_for_app(&self.state.storage.db_pool, &self.app.id)
                .await?
                .into_iter()
                .filter(|active_dep| active_dep.id != self.deployment.id)
                .collect();

        Ok(previous_deployments)
    }

    /// Stops active process containers belonging to previous deployments.
    async fn stop_previous_deployments(&self) -> DockerResult<()> {
        let previous_deployments = self.get_previous_deployments().await?;

        for previous in &previous_deployments {
            let _ = self
                .log
                .send("Stopping previous deployment processes...".to_string())
                .await;

            if let Err(e) = stop_deployment_processes(self.docker_client, previous).await {
                tracing::warn!(
                    app_slug = %self.app.slug,
                    deployment_id = %previous.id,
                    error = ?e,
                    "Failed to stop previous deployment processes before container creation"
                );
            }
        }

        Ok(())
    }

    /// Promotes the new deployment to active status and tears down old containers.
    async fn promote_active(&self) -> DockerResult<()> {
        let db_pool = &self.state.storage.db_pool;

        let previous_deployments = self.get_previous_deployments().await?;

        for previous in &previous_deployments {
            if let Err(e) =
                DeploymentRepo::update_status(db_pool, &previous.id, DeploymentStatus::Stopped)
                    .await
            {
                tracing::warn!(
                    app_slug = %self.app.slug,
                    deployment_id = %previous.id,
                    error = ?e,
                    "Failed to set status to stopped for previous deployment"
                );
            }
        }

        DeploymentRepo::update_status(db_pool, &self.deployment.id, DeploymentStatus::Running)
            .await?;

        self.state.runtime.proxy_sync_trigger.notify_one();

        for previous_deployment in &previous_deployments {
            if let Err(e) =
                remove_deployment_processes(self.docker_client, previous_deployment).await
            {
                tracing::warn!(
                    app_slug = %self.app.slug,
                    deployment_id = %previous_deployment.id,
                    error = ?e,
                    "Failed to remove previous deployment processes"
                );
            }
        }

        if let Err(e) = prune_app_images(self.docker_client, db_pool, self.app).await {
            tracing::warn!(
                app_slug = %self.app.slug,
                error = ?e,
                "Failed to prune old deployment images"
            );
        }

        Ok(())
    }
}
