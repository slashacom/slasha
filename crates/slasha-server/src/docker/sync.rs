use bollard::Docker;
use slasha_db::{
    DbPool,
    deployment::DeploymentStatus,
    repos::{
        app::AppRepo, app_scale::AppScaleRepo, deployment::DeploymentRepo, service::ServiceRepo,
    },
    service::ServiceStatus,
};

use super::{
    deployment::{list_deployment_processes, scale_deployment_process},
    logs::{LogKey, stream_container_logs},
    naming::service_container_name,
};
use crate::state::Runtime;

pub async fn startup_container_sync(
    docker_client: &Docker,
    db_pool: &DbPool,
    runtime: &Runtime,
) -> anyhow::Result<()> {
    let deployments = DeploymentRepo::list_non_terminal(db_pool).await?;
    let services = ServiceRepo::list_non_terminal(db_pool).await?;

    for (svc, app_slug) in services {
        let name = service_container_name(&svc.id);

        if svc.status == ServiceStatus::Provisioning {
            let _ = docker_client
                .remove_container(
                    &name,
                    Some(
                        bollard::query_parameters::RemoveContainerOptionsBuilder::new()
                            .force(true)
                            .build(),
                    ),
                )
                .await;
            ServiceRepo::update_status(db_pool, &svc.id, ServiceStatus::Failed).await?;
        } else if svc.status == ServiceStatus::Running {
            match docker_client.inspect_container(&name, None).await {
                Ok(info) => {
                    if info.state.and_then(|s| s.running) != Some(true) {
                        ServiceRepo::update_status(db_pool, &svc.id, ServiceStatus::Stopped)
                            .await?;
                    } else {
                        let log_key = LogKey::Service {
                            app_slug: app_slug.clone(),
                            service_name: svc.name,
                        };
                        let log = runtime.log_manager.get_logger(&log_key).await?;
                        let docker_client = docker_client.clone();
                        stream_container_logs(
                            docker_client.clone(),
                            log.clone(),
                            name.clone(),
                            None,
                        );
                    }
                }
                Err(_) => {
                    ServiceRepo::update_status(db_pool, &svc.id, ServiceStatus::Failed).await?;
                }
            }
        }
    }

    for deployment in deployments {
        let app = AppRepo::find_by_id(db_pool, &deployment.app_id).await?;

        let app_scales = AppScaleRepo::list_for_app(db_pool, &app.id).await?;

        let log_key = LogKey::Deployment {
            app_slug: app.slug.clone(),
            deployment_id: deployment.id.clone(),
        };
        let log = runtime.log_manager.get_logger(&log_key).await?;

        if deployment.status == DeploymentStatus::Running {
            for scale in app_scales {
                scale_deployment_process(
                    docker_client,
                    db_pool,
                    &runtime.proxy_sync_trigger,
                    &log,
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
