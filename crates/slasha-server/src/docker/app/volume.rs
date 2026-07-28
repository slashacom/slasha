use std::collections::HashMap;

use bollard::{Docker, query_parameters::ListVolumesOptions};
use serde::{Deserialize, Serialize};
use slasha_db::app::App;

use crate::docker::{
    DockerResult, app::deploy::context::MANAGED_DATA_PATH, naming::app_volume_prefix,
};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AppVolume {
    pub path: String,
    pub is_managed: bool,
    pub size_bytes: Option<i64>,
}

/// Lists all persistent volumes associated with an application.
///
/// # Arguments
///
/// * `docker_client` - Docker API client ([`Docker`]).
/// * `app` - Target application model ([`App`]).
///
/// # Returns
///
/// A [`DockerResult`] containing a vector of [`AppVolume`] structs.
pub async fn list_app_volumes(docker_client: &Docker, app: &App) -> DockerResult<Vec<AppVolume>> {
    let prefix = app_volume_prefix(&app.id);

    let mut sizes = HashMap::new();
    if let Ok(usage) = docker_client.df(None).await
        && let Some(items) = usage.volume_usage.and_then(|v| v.items)
    {
        for item in items {
            if let (Some(name), Some(size)) = (
                item.get("Name").and_then(|v| v.as_str()),
                item.pointer("/UsageData/Size").and_then(|v| v.as_i64()),
            ) {
                sizes.insert(name.to_string(), size);
            }
        }
    }

    let response = docker_client
        .list_volumes(Some(ListVolumesOptions {
            filters: Some(HashMap::from([("name".to_string(), vec![prefix.clone()])])),
        }))
        .await?;

    let volumes = response
        .volumes
        .unwrap_or_default()
        .into_iter()
        .filter(|v| v.name.starts_with(&prefix))
        .map(|vol| {
            let path = vol
                .labels
                .get("path")
                .cloned()
                .unwrap_or_else(|| vol.name.clone());

            let is_managed = path == MANAGED_DATA_PATH;
            let size_bytes = sizes.get(&vol.name).copied().filter(|s| *s >= 0);

            AppVolume {
                path,
                is_managed,
                size_bytes,
            }
        })
        .collect();

    Ok(volumes)
}
