use std::collections::HashMap;

use slasha_db::service::{Service, ServiceEnvVar};

use crate::docker::{
    DockerError, DockerResult,
    env_resolver::{RefSource, resolve_env_value, topo_sort_env},
};

/// Resolves and interpolates environment variables for a database service.
///
/// # Arguments
///
/// * `service_vars` - Vector of raw service environment variables ([`ServiceEnvVar`]).
/// * `service` - Target database service model ([`Service`]).
///
/// # Returns
///
/// A [`DockerResult`] containing a [`HashMap`] of key-value environment variables.
pub fn resolve_service_env(
    service_vars: Vec<ServiceEnvVar>,
    service: &Service,
) -> DockerResult<HashMap<String, String>> {
    let raw_map: HashMap<String, String> =
        service_vars.into_iter().map(|v| (v.key, v.value)).collect();

    let sorted = topo_sort_env(&raw_map)?;
    let mut resolved: HashMap<String, String> = HashMap::with_capacity(sorted.len());

    for (key, raw_value) in sorted {
        let value = resolve_env_value(raw_value, |source, ref_key| match source {
            RefSource::Own => resolved.get(ref_key).cloned().ok_or_else(|| {
                DockerError::EnvResolveFailed(format!("Missing variable dependency: {}", ref_key))
            }),
            RefSource::System => match ref_key {
                "service_name" => Ok(service.name.clone()),
                _ => Err(DockerError::EnvResolveFailed(format!(
                    "Unknown system key: {}",
                    ref_key
                ))),
            },
            RefSource::Service(_) => Err(DockerError::EnvResolveFailed(
                "Service references not supported in this context".to_string(),
            )),
        })?;
        resolved.insert(key.to_string(), value);
    }

    Ok(resolved)
}
