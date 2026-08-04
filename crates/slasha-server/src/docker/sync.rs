use futures_util::future::join_all;
use slasha_db::{
    app::App,
    deployment::{Deployment, DeploymentStatus},
    logs::{LogPrefix, ResourceKind},
    models::app_scale::{ProcessStatus, ProcessType},
    repos::{
        app_scale::AppScaleRepo, deployment::DeploymentRepo, node::NodeRepo, service::ServiceRepo,
    },
    service::{Service, ServiceStatus},
};

use super::{
    app::{process::list_deployment_processes, scale::scale_deployment_process},
    naming::service_container_name,
    utils::{self, stream_container_logs},
};
use crate::state::AppState;

/// Reconciles container states across all registered cluster nodes at server startup.
///
/// # Arguments
///
/// * `state` - Application state holding database and runtime handles ([`AppState`]).
///
/// # Returns
///
/// An [`anyhow::Result`] indicating overall synchronization success.
pub async fn startup_container_sync(state: &AppState) -> anyhow::Result<()> {
    let nodes = NodeRepo::list(&state.storage.db_pool).await?;

    let mut futures = Vec::new();

    for node in nodes {
        let state = state.clone();

        futures.push(async move {
            let docker_client = match state.clients.docker_registry.get_client(&node) {
                Ok(c) => c,
                Err(e) => {
                    tracing::warn!(node_id = %node.id, error = ?e, "Failed to connect to node during startup sync");
                    return;
                }
            };

            if let Err(e) = sync_node(&state, &docker_client, &node.id).await {
                tracing::error!(node_id = %node.id, error = ?e, "Node sync failed");
            }
        });
    }

    join_all(futures).await;

    Ok(())
}

/// Reconciles deployments and services on a single cluster node.
///
/// # Arguments
///
/// * `state` - Application state holding database and runtime handles ([`AppState`]).
/// * `docker_client` - Docker API client for the node ([`bollard::Docker`]).
/// * `node_id` - Target node ID string.
///
/// # Returns
///
/// An [`anyhow::Result`] indicating single node synchronization success.
async fn sync_node(
    state: &AppState,
    docker_client: &bollard::Docker,
    node_id: &str,
) -> anyhow::Result<()> {
    let db_pool = &state.storage.db_pool;
    let node_deployments = DeploymentRepo::list_for_node(db_pool, node_id).await?;
    let node_services = ServiceRepo::list_for_node(db_pool, node_id).await?;

    for (app, service) in node_services {
        sync_service(state, docker_client, &app, &service).await;
    }

    for (app, deployment) in node_deployments {
        sync_deployment(state, docker_client, &app, &deployment).await;
    }

    Ok(())
}

/// Reconciles state for a single managed database or service container.
///
/// # Arguments
///
/// * `state` - Application state holding database and runtime handles ([`AppState`]).
/// * `docker_client` - Docker API client for the target node ([`bollard::Docker`]).
/// * `app` - Target application model ([`App`]).
/// * `service` - Target service model ([`Service`]).
async fn sync_service(
    state: &AppState,
    docker_client: &bollard::Docker,
    app: &App,
    service: &Service,
) {
    let db_pool = &state.storage.db_pool;
    let name = service_container_name(&service.id);

    match service.status {
        ServiceStatus::Provisioning => {
            if let Err(e) = utils::remove_container(docker_client, &name).await {
                tracing::warn!(container = %name, error = ?e, "Failed to remove service container");
            }
            let _ = ServiceRepo::update_status(db_pool, &service.id, ServiceStatus::Failed).await;
        }
        ServiceStatus::Running => match docker_client.inspect_container(&name, None).await {
            Ok(info) => {
                if info.state.and_then(|s| s.running) != Some(true) {
                    let _ =
                        ServiceRepo::update_status(db_pool, &service.id, ServiceStatus::Stopped)
                            .await;
                } else {
                    let log_writer = state
                        .runtime
                        .log_bus
                        .writer(ResourceKind::Service, &service.id)
                        .app_id(&app.id)
                        .prefix(LogPrefix::Service);

                    stream_container_logs(docker_client.clone(), log_writer, name);
                }
            }
            Err(_) => {
                let _ =
                    ServiceRepo::update_status(db_pool, &service.id, ServiceStatus::Failed).await;
            }
        },
        ServiceStatus::Stopped | ServiceStatus::Failed => {
            if let Ok(info) = docker_client.inspect_container(&name, None).await
                && info.state.and_then(|s| s.running) == Some(true)
            {
                tracing::info!(
                    container = %name,
                    service_id = %service.id,
                    status = %service.status,
                    "stopping orphaned running container for stopped or failed service"
                );
                let _ = utils::stop_container(docker_client, &name, Some(10)).await;
            }
        }
    }
}

/// Reconciles process containers for a single application deployment.
///
/// # Arguments
///
/// * `state` - Application state holding database and runtime handles ([`AppState`]).
/// * `docker_client` - Docker API client for the target node ([`bollard::Docker`]).
/// * `app` - Target application model ([`App`]).
/// * `deployment` - Target deployment model ([`Deployment`]).
async fn sync_deployment(
    state: &AppState,
    docker_client: &bollard::Docker,
    app: &App,
    deployment: &Deployment,
) {
    let db_pool = &state.storage.db_pool;

    match deployment.status {
        DeploymentStatus::Building | DeploymentStatus::Pending => {
            tracing::warn!(
                app_slug = %app.slug,
                deployment_id = %deployment.id,
                status = %deployment.status,
                "failing orphaned deployment left building or pending across restart"
            );

            if let Ok(containers) = list_deployment_processes(docker_client, &deployment.id).await {
                for container in containers {
                    let _ = utils::remove_container(docker_client, &container.name).await;
                }
            }

            let _ =
                DeploymentRepo::update_status(db_pool, &deployment.id, DeploymentStatus::Failed)
                    .await;
        }
        DeploymentStatus::Running => {
            if let Ok(app_scales) = AppScaleRepo::list_for_app(db_pool, &app.id).await {
                for scale in app_scales {
                    if let Err(e) = scale_deployment_process(
                        state,
                        app,
                        deployment,
                        scale.process_type,
                        scale.desired as u32,
                    )
                    .await
                    {
                        tracing::warn!(
                            app_slug = %app.slug,
                            deployment_id = %deployment.id,
                            error = ?e,
                            "failed to scale deployment process during startup sync"
                        );
                    }
                }
            }

            let log_writer = state
                .runtime
                .log_bus
                .writer(ResourceKind::Deployment, &deployment.id)
                .app_id(&app.id);

            if let Ok(containers) = list_deployment_processes(docker_client, &deployment.id).await {
                for container in containers {
                    let log_prefix = match container.process_type {
                        ProcessType::Web => LogPrefix::Web(container.instance_index),
                        ProcessType::Worker => LogPrefix::Worker(container.instance_index),
                        ProcessType::Release => LogPrefix::System,
                    };

                    let container_log = log_writer.clone().prefix(log_prefix);
                    stream_container_logs(docker_client.clone(), container_log, container.name);
                }
            }
        }
        DeploymentStatus::Stopped | DeploymentStatus::Failed => {
            if let Ok(containers) = list_deployment_processes(docker_client, &deployment.id).await {
                for container in containers {
                    if matches!(container.status, ProcessStatus::Running) {
                        tracing::info!(
                            container = %container.name,
                            app_slug = %app.slug,
                            deployment_id = %deployment.id,
                            status = %deployment.status,
                            "stopping orphaned running container for stopped or failed deployment"
                        );
                        let _ =
                            utils::stop_container(docker_client, &container.name, Some(10)).await;
                    }
                }
            }
        }
    }
}
