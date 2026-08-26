use anyhow::Result;
use serde::Deserialize;
use slasha_db::{deployment::Deployment, service::Service};

use crate::{config::ProjectConfig, http::ApiClient};

/// Resolves the application slug from a command argument or `slasha.toml` in the current directory.
///
/// # Arguments
///
/// * `arg` - Optional explicit slug provided via command line.
///
/// # Returns
///
/// The resolved application slug string.
pub fn resolve_slug(arg: Option<String>) -> Result<String> {
    if let Some(slug) = arg
        && !slug.trim().is_empty()
    {
        return Ok(slug);
    }

    let config = ProjectConfig::load()?;
    if let Some(slug) = config.app
        && !slug.trim().is_empty()
    {
        return Ok(slug);
    }

    anyhow::bail!("Missing app slug and no app specified in slasha.toml");
}

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
/// * `id_arg` - Optional explicit deployment ID.
///
/// # Returns
///
/// The resolved deployment ID string.
pub async fn resolve_deployment_id(
    client: &ApiClient,
    slug: &str,
    id_arg: Option<String>,
) -> Result<String> {
    if let Some(id) = id_arg
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

/// Resolves a service ID by matching an explicit ID or service name.
///
/// # Arguments
///
/// * `client` - Reference to the API client ([`ApiClient`]).
/// * `slug` - Target application slug.
/// * `name_or_id` - Service name or UUID string.
///
/// # Returns
///
/// The resolved service ID string.
pub async fn resolve_service_id(
    client: &ApiClient,
    slug: &str,
    name_or_id: &str,
) -> Result<String> {
    let res: ServiceListResponse = client.get(&format!("/api/apps/{}/services", slug)).await?;

    for service in res.services {
        if service.id == name_or_id || service.name == name_or_id {
            return Ok(service.id);
        }
    }

    anyhow::bail!("Service '{}' not found for app '{}'", name_or_id, slug)
}
