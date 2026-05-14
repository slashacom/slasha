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
    service::executor::{
        ExposureSpec, build_service_container_body, create_start_and_wait_healthy,
        resolve_service_env,
    },
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

pub async fn expose_service(
    docker_client: &Docker,
    db_pool: &DbPool,
    log_manager: &LogManager,
    app: &App,
    service: &Service,
    host_port: u16,
    bind_addr: String,
) -> DeploymentResult<()> {
    let spec = ExposureSpec {
        host_port,
        bind_addr,
    };
    recreate_service_container(docker_client, db_pool, log_manager, app, service, Some(spec)).await
}

pub async fn unexpose_service(
    docker_client: &Docker,
    db_pool: &DbPool,
    log_manager: &LogManager,
    app: &App,
    service: &Service,
) -> DeploymentResult<()> {
    recreate_service_container(docker_client, db_pool, log_manager, app, service, None).await
}

async fn recreate_service_container(
    docker_client: &Docker,
    db_pool: &DbPool,
    log_manager: &LogManager,
    app: &App,
    service: &Service,
    exposure: Option<ExposureSpec>,
) -> DeploymentResult<()> {
    let log_key = LogKey::Service {
        app_slug: app.slug.clone(),
        service_name: service.name.clone(),
    };
    let log = log_manager.get_logger(&log_key).await?;
    let container_name = service_container_name(&service.id);

    ServiceRepo::update_status(db_pool, &service.id, ServiceStatus::Provisioning).await?;

    let _ = docker_client
        .remove_container(
            &container_name,
            Some(RemoveContainerOptionsBuilder::new().force(true).build()),
        )
        .await;

    let env_vars = ServiceRepo::get_env_vars(db_pool, &service.id).await?;
    let resolved = resolve_service_env(env_vars, service)?;

    let body = build_service_container_body(service, app, &resolved, exposure.as_ref());

    if let Err(e) = create_start_and_wait_healthy(docker_client, db_pool, service, body, &log, None)
        .await
    {
        tracing::error!("Service reconfigure failed: {:?}", e);
        let _ = log.send(format!("Service reconfigure failed: {}", e)).await;
        let _ = ServiceRepo::update_status(db_pool, &service.id, ServiceStatus::Failed).await;
        return Err(e);
    }

    Ok(())
}
