use bollard::{Docker, models::NetworkCreateRequest};

use crate::docker::{DockerError, DockerResult, naming::app_network_name};

/// Creates an isolated Docker bridge network for an application.
///
/// # Arguments
///
/// * `docker_client` - Docker API client ([`Docker`]).
/// * `app_id` - Target application ID string.
pub async fn create_app_network(docker_client: &Docker, app_id: &str) -> DockerResult<()> {
    let network_name = app_network_name(app_id);

    let config = NetworkCreateRequest {
        name: network_name,
        driver: Some("bridge".to_string()),
        ..Default::default()
    };

    match docker_client.create_network(config).await {
        Ok(_)
        | Err(bollard::errors::Error::DockerResponseServerError {
            status_code: 404, ..
        }) => Ok(()),

        Err(e) => Err(DockerError::DockerClient(e)),
    }
}

/// Removes an application's isolated Docker bridge network.
///
/// # Arguments
///
/// * `docker_client` - Docker API client ([`Docker`]).
/// * `app_id` - Target application ID string.
pub async fn remove_app_network(docker_client: &Docker, app_id: &str) -> DockerResult<()> {
    let network_name = app_network_name(app_id);

    match docker_client.remove_network(&network_name).await {
        Ok(_)
        | Err(bollard::errors::Error::DockerResponseServerError {
            status_code: 404, ..
        }) => Ok(()),

        Err(e) => Err(DockerError::DockerClient(e)),
    }
}
