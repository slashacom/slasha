use std::collections::HashMap;

use bollard::{
    Docker,
    models::VolumeCreateRequest,
    query_parameters::{
        RemoveContainerOptionsBuilder, RemoveVolumeOptions, StopContainerOptionsBuilder,
    },
};

use crate::docker::{DockerError, DockerResult};

/// Starts a stopped Docker container on a node.
///
/// # Arguments
///
/// * `docker_client` - Docker API client ([`Docker`]).
/// * `name` - Target container name string.
pub async fn start_container(docker_client: &Docker, name: &str) -> DockerResult<()> {
    match docker_client
        .start_container(
            name,
            None::<bollard::query_parameters::StartContainerOptions>,
        )
        .await
    {
        Ok(_) => Ok(()),
        Err(bollard::errors::Error::DockerResponseServerError {
            status_code: 304, ..
        }) => Ok(()),
        Err(e) => Err(DockerError::from(e)),
    }
}

/// Stops a running Docker container on a node.
///
/// # Arguments
///
/// * `docker_client` - Docker API client ([`Docker`]).
/// * `name` - Target container name string.
/// * `timeout_secs` - Optional stop timeout in seconds.
pub async fn stop_container(
    docker_client: &Docker,
    name: &str,
    timeout_secs: Option<i32>,
) -> DockerResult<()> {
    let timeout = timeout_secs.unwrap_or(10);
    let options = StopContainerOptionsBuilder::new().t(timeout).build();

    match docker_client.stop_container(name, Some(options)).await {
        Ok(_) => Ok(()),
        Err(bollard::errors::Error::DockerResponseServerError {
            status_code: 404, ..
        }) => {
            tracing::warn!(container = %name, "Container not found on node during stop operation");
            Ok(())
        }
        Err(e) => Err(DockerError::from(e)),
    }
}

/// Restarts a Docker container on a node.
///
/// # Arguments
///
/// * `docker_client` - Docker API client ([`Docker`]).
/// * `name` - Target container name string.
pub async fn restart_container(docker_client: &Docker, name: &str) -> DockerResult<()> {
    docker_client.restart_container(name, None).await?;
    Ok(())
}

/// Force-removes a Docker container from a node.
///
/// # Arguments
///
/// * `docker_client` - Docker API client ([`Docker`]).
/// * `name` - Target container name string.
pub async fn remove_container(docker_client: &Docker, name: &str) -> DockerResult<()> {
    let options = RemoveContainerOptionsBuilder::new().force(true).build();

    match docker_client.remove_container(name, Some(options)).await {
        Ok(_) => Ok(()),
        Err(bollard::errors::Error::DockerResponseServerError {
            status_code: 404, ..
        }) => {
            tracing::warn!(container = %name, "Container not found on node during remove operation");
            Ok(())
        }
        Err(e) => Err(DockerError::from(e)),
    }
}

/// Creates a named Docker volume on a node with optional labels.
///
/// # Arguments
///
/// * `docker_client` - Docker API client ([`Docker`]).
/// * `name` - Volume name string.
/// * `labels` - Optional label key-value map.
pub async fn create_volume(
    docker_client: &Docker,
    name: &str,
    labels: Option<HashMap<String, String>>,
) -> DockerResult<()> {
    docker_client
        .create_volume(VolumeCreateRequest {
            name: Some(name.to_string()),
            labels,
            ..Default::default()
        })
        .await?;
    Ok(())
}

/// Removes a Docker persistent data volume from a node.
///
/// # Arguments
///
/// * `docker_client` - Docker API client ([`Docker`]).
/// * `name` - Volume name string.
pub async fn remove_volume(docker_client: &Docker, name: &str) -> DockerResult<()> {
    match docker_client
        .remove_volume(name, None::<RemoveVolumeOptions>)
        .await
    {
        Ok(_) => Ok(()),
        Err(bollard::errors::Error::DockerResponseServerError {
            status_code: 404, ..
        }) => {
            tracing::warn!(volume = %name, "Volume not found on node during remove operation");
            Ok(())
        }
        Err(e) => Err(DockerError::from(e)),
    }
}
