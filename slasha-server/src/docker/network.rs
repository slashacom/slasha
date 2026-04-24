use bollard::Docker;
use bollard::models::NetworkCreateRequest;

use super::DeploymentResult;
use crate::error::DeploymentError;

pub fn app_network_name(app_id: &str) -> String {
    format!("slasha-{}", app_id)
}

pub async fn create_app_network(docker: &Docker, app_id: &str) -> DeploymentResult<()> {
    let network_name = app_network_name(app_id);

    let config = NetworkCreateRequest {
        name: network_name,
        driver: Some("bridge".to_string()),
        ..Default::default()
    };

    match docker.create_network(config).await {
        Ok(_) => Ok(()),
        Err(e) => Err(DeploymentError::DockerApi(e).into()),
    }
}

pub async fn delete_app_network(docker: &Docker, app_id: &str) -> DeploymentResult<()> {
    let network_name = app_network_name(app_id);

    match docker.remove_network(&network_name).await {
        Ok(_) => Ok(()),
        Err(e) => Err(DeploymentError::DockerApi(e).into()),
    }
}
