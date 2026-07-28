use std::collections::HashMap;

use slasha_db::{
    app::App,
    models::service::ServiceStatus,
    repos::{app::AppRepo, service::ServiceRepo},
};

use crate::docker::{
    DockerError, DockerResult,
    app::deploy::context::MANAGED_DATA_PATH,
    env_resolver::{RefSource, resolve_env_value, topo_sort_env},
    service::{ServiceKindDockerExt, resolve_service_env},
};

/// Resolves and interpolates environment variables for an application and its attached services.
///
/// # Arguments
///
/// * `db_pool` - Database connection pool reference ([`DbPool`](slasha_db::DbPool)).
/// * `app` - Target application model ([`App`]).
///
/// # Returns
///
/// A [`DockerResult`] containing a [`HashMap`] of key-value environment variables.
pub async fn resolve_app_env(
    db_pool: &slasha_db::DbPool,
    app: &App,
) -> DockerResult<HashMap<String, String>> {
    let app_vars = AppRepo::get_env_vars(db_pool, &app.id).await?;
    let app_services = ServiceRepo::list_for_app(db_pool, &app.id).await?;

    let mut service_env_map: HashMap<String, HashMap<String, String>> = HashMap::new();
    for service in &app_services {
        let vars = ServiceRepo::get_env_vars(db_pool, &service.id).await?;
        let mut map = resolve_service_env(vars, service)?;

        let db_url = service.kind.build_connection_url(&service.name, &map);
        map.insert("DATABASE_URL".to_string(), db_url);

        service_env_map.insert(service.id.clone(), map);
    }

    let raw_app_vars: HashMap<String, String> =
        app_vars.into_iter().map(|v| (v.key, v.value)).collect();

    let sorted_vars = topo_sort_env(&raw_app_vars)?;
    let mut resolved: HashMap<String, String> = HashMap::with_capacity(sorted_vars.len());

    for (key, raw_value) in sorted_vars {
        let value = resolve_env_value(raw_value, |source, ref_key| match source {
            RefSource::Own => resolved.get(ref_key).cloned().ok_or_else(|| {
                DockerError::EnvResolveFailed(format!("Missing variable dependency: {}", ref_key))
            }),

            RefSource::System => match ref_key {
                "app_name" => Ok(app.name.clone()),
                "app_slug" => Ok(app.slug.clone()),
                "managed_data_path" => Ok(MANAGED_DATA_PATH.to_string()),
                _ => Err(DockerError::EnvResolveFailed(format!(
                    "Unknown system key: {}",
                    ref_key
                ))),
            },

            RefSource::Service(service_name) => {
                let service = app_services
                    .iter()
                    .find(|s| &s.name == service_name)
                    .ok_or_else(|| DockerError::ServiceNotFound(service_name.clone()))?;

                if service.status != ServiceStatus::Running {
                    return Err(DockerError::ServiceNotRunning(service_name.clone()));
                }

                match ref_key {
                    "service_name" => Ok(service.name.clone()),
                    _ => service_env_map
                        .get(&service.id)
                        .and_then(|m| m.get(ref_key))
                        .cloned()
                        .ok_or_else(|| {
                            DockerError::EnvResolveFailed(format!(
                                "Service \"{}\" does not export env key \"{}\"",
                                service_name, ref_key
                            ))
                        }),
                }
            }
        })?;

        resolved.insert(key.to_string(), value);
    }

    for service in &app_services {
        if service.status != ServiceStatus::Running {
            continue;
        }
        if let Some(map) = service_env_map.get(&service.id) {
            if !resolved.contains_key("DATABASE_URL")
                && let Some(db_url) = map.get("DATABASE_URL")
            {
                resolved.insert("DATABASE_URL".to_string(), db_url.clone());
            }
            if !resolved.contains_key("REDIS_URL")
                && let Some(redis_url) = map.get("REDIS_URL")
            {
                resolved.insert("REDIS_URL".to_string(), redis_url.clone());
            }
        }
    }

    Ok(resolved)
}
