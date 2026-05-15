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
    service::provision::{
        PortExposure, build_container_body, resolve_env_vars, start_and_wait_healthy,
    },
};

pub async fn stop_service(
    docker: &Docker,
    db_pool: &DbPool,
    log_manager: &LogManager,
    app: &App,
    service: &Service,
) -> DeploymentResult<()> {
    let container_name = service_container_name(&service.id);

    docker
        .stop_container(
            &container_name,
            Some(StopContainerOptionsBuilder::new().t(10).build()),
        )
        .await?;

    ServiceRepo::update_status(db_pool, &service.id, ServiceStatus::Stopped).await?;
    log_manager.remove(&log_key(app, service));

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

    let _ = docker
        .remove_container(
            &container_name,
            Some(RemoveContainerOptionsBuilder::new().force(true).build()),
        )
        .await;

    docker
        .remove_volume(
            &volume_name,
            None::<bollard::query_parameters::RemoveVolumeOptions>,
        )
        .await?;

    ServiceRepo::delete(db_pool, &service.id).await?;
    log_manager.remove(&log_key(app, service));

    Ok(())
}

pub async fn expose_service(
    docker: &Docker,
    db_pool: &DbPool,
    log_manager: &LogManager,
    app: &App,
    service: &Service,
    host_port: u16,
) -> DeploymentResult<()> {
    reconfigure(
        docker,
        db_pool,
        log_manager,
        app,
        service,
        Some(PortExposure { host_port }),
    )
    .await
}

pub async fn unexpose_service(
    docker: &Docker,
    db_pool: &DbPool,
    log_manager: &LogManager,
    app: &App,
    service: &Service,
) -> DeploymentResult<()> {
    reconfigure(docker, db_pool, log_manager, app, service, None).await
}

async fn reconfigure(
    docker: &Docker,
    db_pool: &DbPool,
    log_manager: &LogManager,
    app: &App,
    service: &Service,
    exposure: Option<PortExposure>,
) -> DeploymentResult<()> {
    let log = log_manager.get_logger(&log_key(app, service)).await?;
    let container_name = service_container_name(&service.id);

    ServiceRepo::update_status(db_pool, &service.id, ServiceStatus::Provisioning).await?;

    let _ = docker
        .remove_container(
            &container_name,
            Some(RemoveContainerOptionsBuilder::new().force(true).build()),
        )
        .await;

    let env_vars = ServiceRepo::get_env_vars(db_pool, &service.id).await?;
    let resolved = resolve_env_vars(env_vars, service)?;
    let body = build_container_body(service, app, &resolved, exposure.as_ref());

    if let Err(e) = start_and_wait_healthy(docker, db_pool, service, body, &log, None).await {
        tracing::error!("Service reconfigure failed: {:?}", e);
        let _ = log.send(format!("Reconfigure failed: {}", e)).await;
        let _ = ServiceRepo::update_status(db_pool, &service.id, ServiceStatus::Failed).await;
        return Err(e);
    }

    Ok(())
}

fn log_key(app: &App, service: &Service) -> LogKey {
    LogKey::Service {
        app_slug: app.slug.clone(),
        service_name: service.name.clone(),
    }
}
