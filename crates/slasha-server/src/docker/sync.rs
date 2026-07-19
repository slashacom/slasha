use futures_util::future::join_all;
use slasha_db::{
    DbPool,
    deployment::DeploymentStatus,
    repos::{
        app::AppRepo, app_scale::AppScaleRepo, deployment::DeploymentRepo, node::NodeRepo,
        service::ServiceRepo,
    },
    service::ServiceStatus,
};

use super::{
    deployment::{ScaleDeps, list_deployment_processes, scale_deployment_process},
    naming::service_container_name,
};
use crate::{
    docker::DockerRegistry,
    logs::{LogKey, stream_container_logs},
    state::Runtime,
};

pub async fn startup_container_sync(
    docker_registry: &DockerRegistry,
    db_pool: &DbPool,
    runtime: &Runtime,
) -> anyhow::Result<()> {
    let nodes = NodeRepo::list(db_pool).await?;

    let mut futures = Vec::new();

    for node in nodes {
        let docker_registry = docker_registry.clone();
        let db_pool = db_pool.clone();
        let runtime = runtime.clone();

        futures.push(async move {
            let docker_client = match docker_registry.get_client(&node) {
                Ok(c) => c,
                Err(e) => {
                    tracing::warn!(node_id = %node.id, error = ?e, "Failed to connect to node during startup sync");
                    return;
                }
            };

            if let Err(e) = sync_node(&docker_client, &node.id, &db_pool, &runtime).await {
                tracing::error!(node_id = %node.id, error = ?e, "Node sync failed");
            }
        });
    }

    join_all(futures).await;

    Ok(())
}

async fn sync_node(
    docker_client: &bollard::Docker,
    node_id: &str,
    db_pool: &DbPool,
    runtime: &Runtime,
) -> anyhow::Result<()> {
    let all_deployments = DeploymentRepo::list_non_terminal(db_pool).await?;
    let all_services = ServiceRepo::list_non_terminal(db_pool).await?;

    let mut node_deployments = Vec::new();
    for dep in all_deployments {
        let app = AppRepo::find_by_id(db_pool, &dep.app_id).await?;
        if app.node_id == node_id {
            node_deployments.push((app, dep));
        }
    }

    let mut node_services = Vec::new();
    for (svc, app_slug) in all_services {
        let app = AppRepo::find_by_slug(db_pool, &app_slug).await?;
        if app.node_id == node_id {
            node_services.push((app, svc));
        }
    }

    // reconcile Services
    for (app, svc) in node_services {
        let name = service_container_name(&svc.id);

        if svc.status == ServiceStatus::Provisioning {
            if let Err(e) = docker_client
                .remove_container(
                    &name,
                    Some(
                        bollard::query_parameters::RemoveContainerOptionsBuilder::new()
                            .force(true)
                            .build(),
                    ),
                )
                .await
            {
                tracing::warn!(container = %name, error = ?e, "Failed to remove service container");
            }
            ServiceRepo::update_status(db_pool, &svc.id, ServiceStatus::Failed).await?;
        } else if svc.status == ServiceStatus::Running {
            match docker_client.inspect_container(&name, None).await {
                Ok(info) => {
                    if info.state.and_then(|s| s.running) != Some(true) {
                        ServiceRepo::update_status(db_pool, &svc.id, ServiceStatus::Stopped)
                            .await?;
                    } else {
                        let log_key = LogKey::Service {
                            app_slug: app.slug.clone(),
                            service_name: svc.name,
                        };
                        let log = runtime.log_manager.get_logger(&log_key).await?;
                        stream_container_logs(docker_client.clone(), log.clone(), name, None);
                    }
                }
                Err(_) => {
                    ServiceRepo::update_status(db_pool, &svc.id, ServiceStatus::Failed).await?;
                }
            }
        }
    }

    // reconcile Deployments
    for (app, deployment) in node_deployments {
        let app_scales = AppScaleRepo::list_for_app(db_pool, &app.id).await?;

        let log_key = LogKey::Deployment {
            app_slug: app.slug.clone(),
            deployment_id: deployment.id.clone(),
        };
        let log = runtime.log_manager.get_logger(&log_key).await?;

        if deployment.status == DeploymentStatus::Running {
            for scale in app_scales {
                scale_deployment_process(
                    ScaleDeps {
                        docker_client,
                        db_pool,
                        proxy_sync: &runtime.proxy_sync_trigger,
                        log: &log,
                    },
                    &app,
                    &deployment,
                    scale.process_type,
                    scale.desired as u32,
                    runtime.get_scaling_lock(&deployment.id),
                )
                .await?;
            }

            let containers = list_deployment_processes(docker_client, &deployment.id).await?;
            for container in containers {
                let prefix = format!(
                    "[{}.{}]",
                    container.process_type.to_string().to_lowercase(),
                    container.instance_index
                );
                stream_container_logs(
                    docker_client.clone(),
                    log.clone(),
                    container.name,
                    Some(prefix),
                );
            }
        }
    }

    Ok(())
}
