use std::sync::Arc;

use bollard::{
    Docker,
    query_parameters::{RemoveContainerOptionsBuilder, StopContainerOptionsBuilder},
};
use slasha_db::{
    DbPool,
    app::App,
    repos::service::ServiceRepo,
    service::{Service, ServiceStatus},
};
use tokio::sync::Notify;

use crate::docker::{
    DeploymentResult,
    logs::{LogKey, LogManager, stream_container_logs},
    naming::{service_container_name, service_volume_name},
};

pub async fn stop_service(
    docker: &Docker,
    db_pool: &DbPool,
    log_manager: &LogManager,
    app: &App,
    service: &Service,
) -> DeploymentResult<()> {
    let container_name = service_container_name(&service.id);
    let log_key = LogKey::Service {
        app_slug: app.slug.clone(),
        service_name: service.name.clone(),
    };

    docker
        .stop_container(
            &container_name,
            Some(StopContainerOptionsBuilder::new().t(10).build()),
        )
        .await?;

    ServiceRepo::update_status(db_pool, &service.id, ServiceStatus::Stopped).await?;
    log_manager.remove(&log_key);

    Ok(())
}

pub async fn restart_service(
    docker: &Docker,
    db_pool: &DbPool,
    log_manager: &LogManager,
    proxy_sync_trigger: &Arc<Notify>,
    app: &App,
    service: &Service,
) -> DeploymentResult<()> {
    let container_name = service_container_name(&service.id);
    docker.restart_container(&container_name, None).await?;

    let log_key = LogKey::Service {
        app_slug: app.slug.clone(),
        service_name: service.name.clone(),
    };
    let log = log_manager.get_logger(&log_key).await?;

    stream_container_logs(docker.clone(), log, container_name, None);

    ServiceRepo::update_status(db_pool, &service.id, ServiceStatus::Running).await?;
    proxy_sync_trigger.notify_one();
    Ok(())
}

pub async fn delete_service(
    docker: &Docker,
    db_pool: &DbPool,
    log_manager: &LogManager,
    app: &App,
    service: &Service,
) -> DeploymentResult<()> {
    let container_name = service_container_name(&service.id);
    let volume_name = service_volume_name(&service.id);
    let log_key = LogKey::Service {
        app_slug: app.slug.clone(),
        service_name: service.name.clone(),
    };

    docker
        .remove_container(
            &container_name,
            Some(RemoveContainerOptionsBuilder::new().force(true).build()),
        )
        .await?;

    docker
        .remove_volume(
            &volume_name,
            None::<bollard::query_parameters::RemoveVolumeOptions>,
        )
        .await?;

    ServiceRepo::delete(db_pool, &service.id).await?;
    log_manager.remove(&log_key);

    Ok(())
}
