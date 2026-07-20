use bollard::{
    Docker, query_parameters::{RemoveContainerOptionsBuilder, RemoveVolumeOptions, StopContainerOptionsBuilder},
};
use slasha_db::{
    DbPool,
    app::App,
    repos::service::ServiceRepo,
    service::{Service, ServiceStatus},
};

use crate::{
    docker::{
        DeploymentResult,
        naming::{service_container_name, service_volume_name},
    },
    logs::{LogKey, LogManager, stream_container_logs},
};

/// Stops a running database service container, updates DB status to `Stopped`, and cleans up active loggers.
///
/// # Arguments
/// * `docker` - Docker API client reference.
/// * `db_pool` - Database connection pool reference.
/// * `log_manager` - Service log streaming manager reference.
/// * `app` - Parent app metadata reference.
/// * `service` - Service model reference.
///
/// # Returns
/// A `DeploymentResult` indicating whether the container was successfully stopped.
pub async fn stop_service_container(
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

/// Restarts a database service container, attaches live log streaming, and updates DB status to `Running`.
///
/// # Arguments
/// * `docker` - Docker API client reference.
/// * `db_pool` - Database connection pool reference.
/// * `log_manager` - Service log streaming manager reference.
/// * `app` - Parent app metadata reference.
/// * `service` - Service model reference.
///
/// # Returns
/// A `DeploymentResult` indicating whether the container was successfully restarted.
pub async fn restart_service_container(
    docker: &Docker,
    db_pool: &DbPool,
    log_manager: &LogManager,
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

    Ok(())
}

/// Force-removes a database service container and optionally removes its persistent data volume.
///
/// Note: This function manages Docker infrastructure removal and does not delete the database record.
///
/// # Arguments
/// * `docker` - Docker API client reference.
/// * `log_manager` - Service log streaming manager reference.
/// * `app` - Parent app metadata reference.
/// * `service` - Service model reference.
/// * `remove_volume` - If `true`, removes the associated persistent data volume; if `false`, retains the volume.
///
/// # Returns
/// A `DeploymentResult` indicating successful container and volume cleanup.
pub async fn remove_service_container(
    docker: &Docker,
    log_manager: &LogManager,
    app: &App,
    service: &Service,
    remove_volume: bool,
) -> DeploymentResult<()> {
    let container_name = service_container_name(&service.id);
    let volume_name = service_volume_name(&service.id);
    let log_key = LogKey::Service {
        app_slug: app.slug.clone(),
        service_name: service.name.clone(),
    };

    if let Err(e) = docker
        .remove_container(
            &container_name,
            Some(RemoveContainerOptionsBuilder::new().force(true).build()),
        )
        .await
    {
        tracing::warn!(container = %container_name, error = ?e, "Failed to remove container");
    }

    if remove_volume {
        docker
            .remove_volume(
                &volume_name,
                None::<RemoveVolumeOptions>,
            )
            .await?;
    }

    log_manager.remove(&log_key);

    Ok(())
}
