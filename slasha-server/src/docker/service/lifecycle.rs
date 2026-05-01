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

use crate::docker::{
    DeploymentResult,
    logs::{LogKey, LogManager},
    naming::{service_container_name, service_volume_name},
};

pub async fn stop_service(
    docker_client: &Docker,
    db_pool: &DbPool,
    log_manager: &LogManager,
    app: &App,
    service: &Service,
) -> DeploymentResult<()> {
    let container_name = service_container_name(&service.id);

    docker_client
        .stop_container(
            &container_name,
            Some(StopContainerOptionsBuilder::new().t(10).build()),
        )
        .await?;

    ServiceRepo::update_status(db_pool, &service.id, ServiceStatus::Stopped).await?;

    log_manager.remove(&LogKey::Service {
        app_slug: app.slug.clone(),
        service_name: service.name.clone(),
    });

    Ok(())
}

pub async fn delete_service(
    docker_client: &Docker,
    db_pool: &DbPool,
    log_manager: &LogManager,
    app: &App,
    service: &Service,
) -> DeploymentResult<()> {
    let container_name = service_container_name(&service.id);
    let volume_name = service_volume_name(&service.id);

    docker_client
        .remove_container(
            &container_name,
            Some(RemoveContainerOptionsBuilder::new().force(true).build()),
        )
        .await?;

    docker_client
        .remove_volume(
            &volume_name,
            None::<bollard::query_parameters::RemoveVolumeOptions>,
        )
        .await?;

    ServiceRepo::delete(db_pool, &service.id).await?;

    log_manager.remove(&LogKey::Service {
        app_slug: app.slug.clone(),
        service_name: service.name.clone(),
    });

    Ok(())
}
