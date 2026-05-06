use std::sync::Arc;

use bollard::Docker;
use slasha_db::{
    DbPool,
    deployment::DeploymentStatus,
    repos::{deployment::DeploymentRepo, service::ServiceRepo},
    service::ServiceStatus,
};

use super::{
    logs::{LogKey, LogManager, stream_container_logs},
    naming::{app_container_name, service_container_name},
};

enum ContainerRecord {
    Deployment {
        id: String,
        app_id: String,
        app_slug: String,
        status: DeploymentStatus,
    },
    Service {
        id: String,
        app_slug: String,
        name: String,
        status: ServiceStatus,
    },
}

impl ContainerRecord {
    fn container_name(&self) -> String {
        match self {
            Self::Deployment { id, app_id, .. } => app_container_name(app_id, id),
            Self::Service { id, .. } => service_container_name(id),
        }
    }

    fn is_pending(&self) -> bool {
        matches!(
            self,
            Self::Deployment {
                status: DeploymentStatus::Pending | DeploymentStatus::Building,
                ..
            } | Self::Service {
                status: ServiceStatus::Provisioning,
                ..
            }
        )
    }

    fn is_running(&self) -> bool {
        matches!(
            self,
            Self::Deployment {
                status: DeploymentStatus::Running,
                ..
            } | Self::Service {
                status: ServiceStatus::Running,
                ..
            }
        )
    }

    fn log_key(&self) -> LogKey {
        match self {
            Self::Deployment { app_slug, id, .. } => LogKey::Deployment {
                app_slug: app_slug.clone(),
                deployment_id: id.clone(),
            },
            Self::Service { app_slug, name, .. } => LogKey::Service {
                app_slug: app_slug.clone(),
                service_name: name.clone(),
            },
        }
    }

    async fn mark_failed(&self, db_pool: &DbPool) -> anyhow::Result<()> {
        match self {
            Self::Deployment { id, .. } => {
                DeploymentRepo::update_status(db_pool, id, DeploymentStatus::Failed).await?;
                Ok(())
            }
            Self::Service { id, .. } => {
                ServiceRepo::update_status(db_pool, id, ServiceStatus::Failed).await?;
                Ok(())
            }
        }
    }

    async fn mark_stopped(&self, db_pool: &DbPool) -> anyhow::Result<()> {
        match self {
            Self::Deployment { id, .. } => {
                DeploymentRepo::update_status(db_pool, id, DeploymentStatus::Stopped).await?;
                Ok(())
            }
            Self::Service { id, .. } => {
                ServiceRepo::update_status(db_pool, id, ServiceStatus::Stopped).await?;
                Ok(())
            }
        }
    }
}

pub async fn run_container_sync(
    docker: &Docker,
    db_pool: &DbPool,
    log_manager: &Arc<LogManager>,
) -> anyhow::Result<()> {
    let deployments = DeploymentRepo::list_non_terminal(db_pool).await?;
    let services = ServiceRepo::list_non_terminal(db_pool).await?;
    let (n_deps, n_svcs) = (deployments.len(), services.len());

    let container_records: Vec<ContainerRecord> = deployments
        .into_iter()
        .map(|(dep, app_slug)| ContainerRecord::Deployment {
            id: dep.id,
            app_id: dep.app_id,
            app_slug,
            status: dep.status,
        })
        .chain(
            services
                .into_iter()
                .map(|(svc, app_slug)| ContainerRecord::Service {
                    id: svc.id,
                    app_slug,
                    name: svc.name,
                    status: svc.status,
                }),
        )
        .collect();

    for container_record in container_records {
        let container_name = container_record.container_name();

        async {
            if container_record.is_pending() {
                docker
                    .remove_container(
                        &container_name,
                        Some(
                            bollard::query_parameters::RemoveContainerOptionsBuilder::new()
                                .force(true)
                                .build(),
                        ),
                    )
                    .await?;
                container_record.mark_failed(db_pool).await?;
            } else if container_record.is_running() {
                match docker.inspect_container(&container_name, None).await {
                    Ok(info) => {
                        let is_running = info.state.and_then(|s| s.running) == Some(true);
                        if !is_running {
                            container_record.mark_stopped(db_pool).await?;
                            return Ok(());
                        }
                    }

                    Err(bollard::errors::Error::DockerResponseServerError {
                        status_code: 404,
                        ..
                    }) => {
                        container_record.mark_failed(db_pool).await?;
                        return Ok(());
                    }

                    Err(e) => anyhow::bail!(e),
                }

                let log = log_manager.get_logger(&container_record.log_key()).await?;
                let docker = docker.clone();
                tokio::spawn(async move {
                    if let Err(e) = stream_container_logs(docker, log, container_name).await {
                        tracing::warn!("Container log stream ended: {:?}", e);
                    }
                });
            }

            Ok(())
        }
        .await?;
    }

    tracing::info!("Reconciled {} deployments, {} services", n_deps, n_svcs);
    Ok(())
}
