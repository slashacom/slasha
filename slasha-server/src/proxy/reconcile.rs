use std::{collections::HashMap, sync::Arc};

use bollard::query_parameters::ListContainersOptionsBuilder;
use tokio::{
    sync::Notify,
    time::{Duration, sleep},
};

use super::{ProxyResult, RouteEntry};
use crate::state::{Clients, Config};

pub async fn reconcile(clients: &Clients, config: &Config) -> ProxyResult<()> {
    let platform_domain = match &config.platform_domain {
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

    let containers = clients.docker.list_containers(Some(opts)).await?;
    let mut routes = Vec::new();
    if !config.private_mode {
        routes.push(RouteEntry {
            domain: platform_domain.clone(),
            upstream_port: 3000,
        });
    }

    for container in containers {
        let labels = match &container.labels {
            Some(l) => l,
            None => continue,
        };

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

    clients.caddy.sync_routes(&routes, config.env).await?;

    tracing::info!("Reconciled {} proxy routes", routes.len());

    Ok(())
}

pub fn spawn_reconciler(clients: Clients, config: Config) -> Arc<Notify> {
    let notify = Arc::new(Notify::new());
    let notify_clone = notify.clone();

    tokio::spawn(async move {
        loop {
            notify_clone.notified().await;

            loop {
                tokio::select! {
                    _ = sleep(Duration::from_millis(500)) => {
                        if let Err(e) = reconcile(&clients, &config).await {
                            tracing::error!("Proxy reconciliation failed: {:?}", e);
                        }
                        break;
                    }
                    _ = notify_clone.notified() => {}
                }
            }
        }
    });

    notify
}
