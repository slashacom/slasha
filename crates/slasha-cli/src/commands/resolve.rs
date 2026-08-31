use anyhow::Result;
use serde::Deserialize;
use slasha_db::{deployment::Deployment, service::Service};

use crate::http::ApiClient;

#[derive(Deserialize)]
struct DeploymentListResponse {
    deployments: Vec<Deployment>,
}

/// Resolves a deployment ID from an explicit argument or defaults to the latest deployment.
///
/// # Arguments
///
/// * `client` - Reference to the API client ([`ApiClient`]).
/// * `slug` - Target application slug.
/// * `deployment_id` - Optional explicit deployment ID.
///
/// # Returns
///
/// The resolved deployment ID string.
pub async fn resolve_deployment_id(
    client: &ApiClient,
    slug: &str,
    deployment_id: Option<String>,
) -> Result<String> {
    if let Some(id) = deployment_id
        && !id.trim().is_empty()
    {
        return Ok(id);
    }

    let res: DeploymentListResponse = client
        .get(&format!("/api/apps/{}/deployments", slug))
        .await?;

    let target_dep = res.deployments.into_iter().max_by_key(|d| d.created_at);

    match target_dep {
        Some(dep) => Ok(dep.id),
        None => anyhow::bail!("No deployments found for app '{}'", slug),
    }
}

#[derive(Deserialize)]
struct ServiceListResponse {
    services: Vec<Service>,
}

/// Resolves a service ID by matching the service name.
///
/// # Arguments
///
/// * `client` - Reference to the API client ([`ApiClient`]).
/// * `slug` - Target application slug.
/// * `name` - Service name.
///
/// # Returns
///
/// The resolved service ID string.
pub async fn resolve_service_id(client: &ApiClient, slug: &str, name: &str) -> Result<String> {
    let res: ServiceListResponse = client.get(&format!("/api/apps/{}/services", slug)).await?;

    for service in res.services {
        if service.name.eq_ignore_ascii_case(name) {
            return Ok(service.id);
        }
    }

    anyhow::bail!("Service '{}' not found for app '{}'", name, slug)
}
