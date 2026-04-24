use super::config::build_config;
use super::{ProxyResult, RouteEntry};
use crate::AppState;
use bollard::query_parameters::ListContainersOptionsBuilder;
use std::collections::HashMap;

pub async fn reconcile(state: &AppState) -> ProxyResult<()> {
    let _guard = state.reconcile_lock.lock().await;

    let platform_domain = match &state.platform_domain {
        Some(d) => d,
        None => {
            tracing::warn!("SLASHA_PLATFORM_DOMAIN not set, skipping proxy reconciliation");
            return Ok(());
        }
    };

    let mut filters: HashMap<String, Vec<String>> = HashMap::new();
    filters.insert("label".to_string(), vec!["slasha.managed=true".to_string()]);
    filters.insert("status".to_string(), vec!["running".to_string()]);

    let opts = ListContainersOptionsBuilder::new()
        .all(true)
        .filters(&filters)
        .build();

    let containers = state.docker.list_containers(Some(opts)).await?;

    let mut routes = Vec::new();

    for container in containers {
        let labels = match &container.labels {
            Some(l) => l,
            None => continue,
        };

        // skip proxy container
        if labels.get("slasha.role").map(|v| v.as_str()) == Some("proxy") {
            continue;
        }

        let app_slug = match labels.get("slasha.app_slug") {
            Some(id) => id,
            None => continue,
        };

        let host_port = match labels.get("slasha.host_port") {
            Some(p) => p.parse::<u16>().unwrap(),
            None => continue,
        };

        routes.push(RouteEntry {
            domain: format!("{}.{}", app_slug, platform_domain),
            upstream_port: host_port,
        });
    }

    let config = build_config(&routes);
    state.caddy_client.load(&config).await?;

    tracing::info!("Reconciled {} proxy routes", routes.len());
    tracing::info!("Config: {:#?}", config);

    Ok(())
}
