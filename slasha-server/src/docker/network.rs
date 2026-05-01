use bollard::{Docker, models::NetworkCreateRequest};

use super::{DeploymentError, DeploymentResult, naming::app_network_name};

pub async fn create_app_network(docker_client: &Docker, app_id: &str) -> DeploymentResult<()> {
    let network_name = app_network_name(app_id);

    let config = NetworkCreateRequest {
        name: network_name,
        driver: Some("bridge".to_string()),
        ..Default::default()
    };

    match docker_client.create_network(config).await {
        Ok(_) => Ok(()),
        Err(e) => Err(DeploymentError::DockerApi(e)),
    }
}

pub async fn delete_app_network(docker_client: &Docker, app_id: &str) -> DeploymentResult<()> {
    let network_name = app_network_name(app_id);

    match docker_client.remove_network(&network_name).await {
        Ok(_) => Ok(()),
        Err(e) => Err(DeploymentError::DockerApi(e)),
    }
}
