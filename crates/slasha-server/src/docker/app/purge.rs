use std::collections::HashMap;

use bollard::{
    Docker,
    query_parameters::{ListContainersOptions, ListVolumesOptions},
};
use slasha_db::{app::App, repos::service::ServiceRepo};

use crate::{
    docker::{
        DockerResult,
        app::{image::remove_app_images, network::remove_app_network},
        labels::LABEL_APP_ID,
        naming::{app_volume_prefix, service_volume_name},
        utils,
    },
    state::Storage,
};

/// Purges all containers, networks, volumes, images, and service data for an application from a node.
///
/// # Arguments
///
/// * `app` - Target application model ([`App`]).
/// * `docker_client` - Docker API client ([`Docker`]).
/// * `storage` - Server storage handle ([`Storage`]).
pub async fn purge_app_from_node(
    app: &App,
    docker_client: &Docker,
    storage: &Storage,
) -> DockerResult<()> {
    remove_app_containers(docker_client, &app.id).await?;
    remove_app_network(docker_client, &app.id).await?;
    remove_app_volumes(docker_client, &app.id).await?;
    remove_app_images(docker_client, &app.slug).await?;

    if let Ok(services) = ServiceRepo::list_for_app(&storage.db_pool, &app.id).await {
        for service in services {
            let volume_name = service_volume_name(&service.id);
            if let Err(e) = utils::remove_volume(docker_client, &volume_name).await {
                tracing::warn!(volume = %volume_name, error = ?e, "Failed to remove service volume during purge");
            }
        }
    }

    Ok(())
}

/// Force removes all Docker containers associated with an application ID across all deployments.
///
/// # Arguments
///
/// * `docker_client` - Docker API client ([`Docker`]).
/// * `app_id` - Target application ID string.
async fn remove_app_containers(docker_client: &Docker, app_id: &str) -> DockerResult<()> {
    let mut filters = HashMap::new();
    filters.insert(
        "label".to_string(),
        vec![format!("{}={}", LABEL_APP_ID, app_id)],
    );

    let containers = docker_client
        .list_containers(Some(ListContainersOptions {
            all: true,
            filters: Some(filters),
            ..Default::default()
        }))
        .await?;

    for container in containers {
        if let Some(id) = container.id
            && let Err(e) = utils::remove_container(docker_client, &id).await
        {
            tracing::warn!(container = %id, error = ?e, "Failed to remove container during purge");
        }
    }

    Ok(())
}

/// Deletes all Docker volumes matching the application's volume prefix.
///
/// # Arguments
///
/// * `docker_client` - Docker API client ([`Docker`]).
/// * `app_id` - Target application ID string.
async fn remove_app_volumes(docker_client: &Docker, app_id: &str) -> DockerResult<()> {
    let prefix = app_volume_prefix(app_id);

    let mut filters: HashMap<String, Vec<String>> = HashMap::new();
    filters.insert("name".to_string(), vec![prefix.clone()]);

    let response = docker_client
        .list_volumes(Some(ListVolumesOptions {
            filters: Some(filters),
        }))
        .await?;

    let names: Vec<String> = response
        .volumes
        .unwrap_or_default()
        .into_iter()
        .map(|v| v.name)
        .filter(|n| n.starts_with(&prefix))
        .collect();

    for name in names {
        if let Err(e) = utils::remove_volume(docker_client, &name).await {
            tracing::warn!(volume = %name, error = ?e, "Failed to remove app volume during purge");
        }
    }

    Ok(())
}
