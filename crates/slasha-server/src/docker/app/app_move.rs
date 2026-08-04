use std::collections::HashMap;

use bollard::{
    Docker, body_try_stream,
    models::{ContainerCreateBody, HostConfig, Mount, MountType},
    query_parameters::{
        CreateContainerOptions, CreateImageOptions, DownloadFromContainerOptions,
        ListVolumesOptions, UploadToContainerOptions,
    },
};
use futures_util::{StreamExt, TryStreamExt};
use slasha_db::{
    app::App,
    deployment::Deployment,
    models::node::Node,
    repos::{app::AppRepo, deployment::DeploymentRepo, service::ServiceRepo},
    service::ServiceStatus,
};

use super::process::restart_deployment_processes;
use crate::{
    docker::{
        DockerError, DockerResult,
        app::{
            deploy,
            network::{create_app_network, remove_app_network},
            process::stop_deployment_processes,
            purge::purge_app_from_node,
        },
        naming::{app_volume_prefix, service_volume_name},
        service::run_provision_service_workflow,
        service_container_name, utils,
        workflow::{WorkflowRunner, runner::WorkflowContext},
    },
    operations,
    state::AppState,
};

const HELPER_IMAGE: &str = "alpine:latest";

/// Pulls the helper Alpine Docker image if not present on the host.
///
/// # Arguments
///
/// * `docker_client` - Docker API client ([`Docker`]).
async fn ensure_alpine_image(docker_client: &Docker) -> DockerResult<()> {
    let mut stream = docker_client.create_image(
        Some(CreateImageOptions {
            from_image: Some(HELPER_IMAGE.to_string()),
            ..Default::default()
        }),
        None,
        None,
    );

    while let Some(item) = stream.next().await {
        item?;
    }

    Ok(())
}

/// Creates a helper container with a volume mounted for data transfer.
///
/// # Arguments
///
/// * `docker_client` - Docker API client ([`Docker`]).
/// * `container_name` - Name for the helper container.
/// * `volume_name` - Name of the volume to mount.
async fn create_helper_container(
    docker_client: &Docker,
    container_name: &str,
    volume_name: &str,
) -> DockerResult<()> {
    let body = ContainerCreateBody {
        image: Some(HELPER_IMAGE.to_string()),
        host_config: Some(HostConfig {
            mounts: Some(vec![Mount {
                typ: Some(MountType::VOLUME),
                source: Some(volume_name.to_string()),
                target: Some("/volume_data".to_string()),
                ..Default::default()
            }]),
            ..Default::default()
        }),
        ..Default::default()
    };

    docker_client
        .create_container(
            Some(CreateContainerOptions {
                name: Some(container_name.to_string()),
                ..Default::default()
            }),
            body,
        )
        .await?;
    Ok(())
}

pub struct AppMoveRunner<'a> {
    pub state: &'a AppState,
    pub app: &'a App,
    pub source_docker_client: &'a Docker,
    pub target_docker_client: &'a Docker,
    pub target_node: &'a Node,
    pub wf: &'a WorkflowContext<'a>,
}

impl<'a> AppMoveRunner<'a> {
    /// Executes the application node migration workflow step by step.
    ///
    /// # Arguments
    ///
    /// * `was_running` - Whether the application had active running containers before migration.
    /// * `active_deployments` - Slice of active deployment models ([`Deployment`]).
    pub async fn execute(
        &self,
        was_running: bool,
        active_deployments: &[Deployment],
    ) -> DockerResult<()> {
        if was_running {
            self.wf
                .step(
                    "Stopping application containers on source host",
                    async {
                        for dep in active_deployments {
                            stop_deployment_processes(self.source_docker_client, dep).await?;
                        }
                        Ok::<(), DockerError>(())
                    },
                    {
                        let source_docker_client = self.source_docker_client.clone();
                        let runtime = self.state.runtime.clone();
                        let app = self.app.clone();
                        let active_deployments = active_deployments.to_vec();
                        async move {
                            for dep in &active_deployments {
                                let _ = restart_deployment_processes(
                                    &source_docker_client,
                                    &runtime,
                                    &app,
                                    dep,
                                )
                                .await;
                            }
                        }
                    },
                )
                .await?;
        }

        let app_services =
            ServiceRepo::list_for_app(&self.state.storage.db_pool, &self.app.id).await?;

        let running_services: Vec<_> = app_services
            .into_iter()
            .filter(|s| s.status == ServiceStatus::Running)
            .collect();

        if !running_services.is_empty() {
            self.wf
                .step(
                    "Stopping database services on source host",
                    async {
                        for service in &running_services {
                            let container_name = service_container_name(&service.id);

                            utils::stop_container(
                                self.source_docker_client,
                                &container_name,
                                Some(10),
                            )
                            .await?;
                        }
                        Ok::<(), DockerError>(())
                    },
                    {
                        let source_docker_client = self.source_docker_client.clone();
                        let running_services = running_services.clone();
                        async move {
                            for service in &running_services {
                                let container_name = service_container_name(&service.id);
                                let _ =
                                    utils::start_container(&source_docker_client, &container_name)
                                        .await;
                            }
                        }
                    },
                )
                .await?;
        }

        self.migrate_app_and_service_volumes().await?;

        self.wf
            .step(
                "Updating host node assignment in database",
                async {
                    AppRepo::update_node(
                        &self.state.storage.db_pool,
                        &self.app.id,
                        &self.target_node.id,
                    )
                    .await?;
                    Ok::<(), DockerError>(())
                },
                {
                    let db_pool = self.state.storage.db_pool.clone();
                    let app_id = self.app.id.clone();
                    let source_node_id = self.app.node_id.clone();
                    async move {
                        let _ = AppRepo::update_node(&db_pool, &app_id, &source_node_id).await;
                    }
                },
            )
            .await?;

        self.wf
            .step(
                "Creating application network on target host",
                create_app_network(self.target_docker_client, &self.app.id),
                {
                    let target_docker_client = self.target_docker_client.clone();
                    let app_id = self.app.id.clone();
                    async move {
                        let _ = remove_app_network(&target_docker_client, &app_id).await;
                    }
                },
            )
            .await?;

        self.reprovision_services().await?;

        if was_running && let Some(deployment) = active_deployments.first() {
            let mut target_app = self.app.clone();
            target_app.node_id = self.target_node.id.clone();

            self.wf
                .step(
                    "Starting application on target host",
                    deploy::run_deployment_workflow(
                        self.state.clone(),
                        target_app,
                        self.target_docker_client.clone(),
                        deployment.clone(),
                        None,
                        tokio_util::sync::CancellationToken::new(),
                    ),
                    async {},
                )
                .await?;
        }

        Ok(())
    }

    /// Streams data for a single volume from the source host to the target host.
    ///
    /// # Arguments
    ///
    /// * `volume_name` - Name of the volume to migrate.
    async fn migrate_single_volume(&self, volume_name: &str) -> DockerResult<()> {
        let helper_src_name = format!("slasha-move-src-{}", uuid::Uuid::new_v4());
        let helper_dst_name = format!("slasha-move-dst-{}", uuid::Uuid::new_v4());

        utils::create_volume(self.target_docker_client, volume_name, None).await?;

        create_helper_container(self.source_docker_client, &helper_src_name, volume_name).await?;
        create_helper_container(self.target_docker_client, &helper_dst_name, volume_name).await?;

        let stream = self
            .source_docker_client
            .download_from_container(
                &helper_src_name,
                Some(DownloadFromContainerOptions {
                    path: "/volume_data".to_string(),
                }),
            )
            .map_err(std::io::Error::other);

        let body = body_try_stream(stream);
        let upload_res = self
            .target_docker_client
            .upload_to_container(
                &helper_dst_name,
                Some(UploadToContainerOptions {
                    path: "/".to_string(),
                    no_overwrite_dir_non_dir: Some("false".to_string()),
                    ..Default::default()
                }),
                body,
            )
            .await;

        if let Err(e) = utils::remove_container(self.source_docker_client, &helper_src_name).await {
            tracing::warn!(container = %helper_src_name, error = ?e, "Failed to remove source helper container during move");
        }

        if let Err(e) = utils::remove_container(self.target_docker_client, &helper_dst_name).await {
            tracing::warn!(container = %helper_dst_name, error = ?e, "Failed to remove target helper container during move");
        }

        upload_res?;

        Ok(())
    }

    /// Migrates all application and attached database service volumes to the target host.
    async fn migrate_app_and_service_volumes(&self) -> DockerResult<()> {
        let prefix = app_volume_prefix(&self.app.id);

        let mut list_filters = HashMap::new();
        list_filters.insert("name".to_string(), vec![prefix.clone()]);

        let response = self
            .source_docker_client
            .list_volumes(Some(ListVolumesOptions {
                filters: Some(list_filters),
            }))
            .await?;

        let volumes: Vec<String> = response
            .volumes
            .unwrap_or_default()
            .into_iter()
            .map(|v| v.name)
            .filter(|n| n.starts_with(&prefix))
            .collect();

        for volume_name in &volumes {
            self.wf
                .step(
                    format!("Migrating application volume {}", volume_name),
                    self.migrate_single_volume(volume_name),
                    {
                        let target_docker_client = self.target_docker_client.clone();
                        let volume_name = volume_name.clone();
                        async move {
                            let _ = utils::remove_volume(&target_docker_client, &volume_name).await;
                        }
                    },
                )
                .await?;
        }

        let app_services =
            ServiceRepo::list_for_app(&self.state.storage.db_pool, &self.app.id).await?;
        for service in &app_services {
            let volume_name = service_volume_name(&service.id);

            self.wf
                .step(
                    format!("Migrating database service volume {}", volume_name),
                    self.migrate_single_volume(&volume_name),
                    {
                        let target_docker_client = self.target_docker_client.clone();
                        let volume_name = volume_name.clone();
                        async move {
                            let _ = utils::remove_volume(&target_docker_client, &volume_name).await;
                        }
                    },
                )
                .await?;
        }

        Ok(())
    }

    /// Re-provisions attached database service containers on the target host.
    async fn reprovision_services(&self) -> DockerResult<()> {
        let mut target_app = self.app.clone();
        target_app.node_id = self.target_node.id.clone();

        let app_services =
            ServiceRepo::list_for_app(&self.state.storage.db_pool, &target_app.id).await?;

        if app_services.is_empty() {
            return Ok(());
        }

        self.wf
            .step(
                "Re-provisioning database services on target host",
                async {
                    let service_futures = app_services.into_iter().map(|service| {
                        let target_docker_client = self.target_docker_client.clone();
                        let state = self.state.clone();
                        let target_app = target_app.clone();

                        async move {
                            let _guard = state.runtime.operations.try_acquire_service(
                                &service.id,
                                operations::ServiceOperation::Provisioning,
                            )?;

                            run_provision_service_workflow(
                                state,
                                target_app,
                                target_docker_client,
                                service,
                                true,
                            )
                            .await
                        }
                    });

                    futures_util::future::try_join_all(service_futures).await?;
                    Ok(())
                },
                async {},
            )
            .await
    }
}

/// Runs the background workflow to migrate an application and its resources to a target node.
///
/// # Arguments
///
/// * `state` - Application state holding database and runtime handles ([`AppState`]).
/// * `app` - Target application model ([`App`]).
/// * `source_docker_client` - Source node Docker API client ([`Docker`]).
/// * `target_docker_client` - Destination node Docker API client ([`Docker`]).
/// * `target_node` - Destination node model ([`Node`]).
/// * `guard` - Operation guard for migration ([`OperationGuard`](operations::OperationGuard)).
pub async fn run_move_app_workflow(
    state: AppState,
    app: App,
    source_docker_client: Docker,
    target_docker_client: Docker,
    target_node: Node,
) -> DockerResult<()> {
    let db_pool = &state.storage.db_pool;

    ensure_alpine_image(&source_docker_client).await?;
    ensure_alpine_image(&target_docker_client).await?;

    let active_deployments = DeploymentRepo::list_active_for_app(db_pool, &app.id).await?;
    let was_running = !active_deployments.is_empty();

    let res = WorkflowRunner::new(format!("move_app:{}:to:{}", app.slug, target_node.name))
        .run({
            let state = state.clone();
            let app = app.clone();
            let source_docker_client = source_docker_client.clone();
            let target_docker_client = target_docker_client.clone();
            let target_node = target_node.clone();

            move |wf| async move {
                let runner = AppMoveRunner {
                    state: &state,
                    app: &app,
                    source_docker_client: &source_docker_client,
                    target_docker_client: &target_docker_client,
                    target_node: &target_node,
                    wf: &wf,
                };

                runner.execute(was_running, &active_deployments).await
            }
        })
        .await;

    if let Err(e) = &res {
        tracing::error!(
            app_slug = %app.slug,
            error = ?e,
            "node migration failed"
        );

        return Ok(());
    }

    if let Err(e) = purge_app_from_node(&app, &source_docker_client, &state.storage).await {
        tracing::warn!(app_id = %app.id, error = ?e, "Failed to purge app from old node");
    }

    Ok(())
}
