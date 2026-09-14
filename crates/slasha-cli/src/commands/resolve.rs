use anyhow::Result;
use serde::Deserialize;
use slasha_db::{
    deployment::{Deployment, DeploymentStatus},
    models::service_backup::{ServiceBackup, ServiceBackupStatus},
    service::Service,
};

use crate::http::ApiClient;

#[derive(Deserialize)]
struct DeploymentListResponse {
    deployments: Vec<Deployment>,
}

/// Resolves the latest running deployment ID for an application.
///
/// # Arguments
///
/// * `client` - Reference to the API client ([`ApiClient`]).
/// * `slug` - Target application slug.
///
/// # Returns
///
/// The resolved running deployment ID string.
pub async fn resolve_running_deployment_id(client: &ApiClient, slug: &str) -> Result<String> {
    let res: DeploymentListResponse = client
        .get(&format!("/api/apps/{}/deployments", slug))
        .await?;

    let running_dep = res
        .deployments
        .into_iter()
        .filter(|d| d.status == DeploymentStatus::Running)
        .max_by_key(|d| d.created_at);

    match running_dep {
        Some(dep) => Ok(dep.id),
        None => anyhow::bail!("No running deployment found for app '{}'", slug),
    }
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
struct ServiceListItem {
    service: Service,
}

#[derive(Deserialize)]
struct ServiceListResponse {
    services: Vec<ServiceListItem>,
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

    for item in res.services {
        if item.service.name.eq_ignore_ascii_case(name) {
            return Ok(item.service.id);
        }
    }

    anyhow::bail!("Service '{}' not found for app '{}'", name, slug)
}

#[derive(Deserialize)]
struct NodeListItem {
    id: String,
    name: String,
}

#[derive(Deserialize)]
struct NodeListResponse {
    nodes: Vec<NodeListItem>,
}

/// Resolves a node ID by matching the node name.
///
/// # Arguments
///
/// * `client` - Reference to the API client ([`ApiClient`]).
/// * `name` - Target node name string slice.
///
/// # Returns
///
/// The resolved node ID string.
pub async fn resolve_node_id(client: &ApiClient, name: &str) -> Result<String> {
    let res: NodeListResponse = client.get("/api/nodes").await?;

    for node in res.nodes {
        if node.name.eq_ignore_ascii_case(name) {
            return Ok(node.id);
        }
    }

    anyhow::bail!("Node '{}' not found", name)
}

#[derive(Deserialize)]
struct BackupListResponse {
    backups: Vec<ServiceBackup>,
}

/// Resolves a service backup by file name or defaults to the latest completed backup.
///
/// # Arguments
///
/// * `client` - Reference to the API client ([`ApiClient`]).
/// * `slug` - Target application slug.
/// * `service_id` - Target service ID string.
/// * `service_name` - Service name string for error reporting.
/// * `reference` - Optional backup file name or 'latest'.
///
/// # Returns
///
/// The resolved [`ServiceBackup`].
pub async fn resolve_backup(
    client: &ApiClient,
    slug: &str,
    service_id: &str,
    service_name: &str,
    reference: Option<&str>,
) -> Result<ServiceBackup> {
    let res: BackupListResponse = client
        .get(&format!(
            "/api/apps/{}/services/{}/backups",
            slug, service_id
        ))
        .await?;

    let reference = reference.map(str::trim).filter(|s| !s.is_empty());

    match reference {
        None | Some("latest") => {
            let target = res
                .backups
                .into_iter()
                .filter(|b| b.status == ServiceBackupStatus::Succeeded)
                .max_by_key(|b| b.created_at);

            match target {
                Some(b) => Ok(b),
                None => anyhow::bail!("No completed backups found for service '{}'", service_name),
            }
        }
        Some(target) => res
            .backups
            .into_iter()
            .find(|b| b.file_name == target)
            .ok_or_else(|| {
                anyhow::anyhow!(
                    "Backup '{}' not found for service '{}'",
                    target,
                    service_name
                )
            }),
    }
}
