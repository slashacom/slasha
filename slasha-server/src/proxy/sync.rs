use std::{collections::HashMap, sync::Arc};

use bollard::query_parameters::ListContainersOptionsBuilder;
use tokio::{
    sync::Notify,
    time::{Duration, sleep},
};

use super::{PROXY_NETWORK_NAME, ProxyResult, RouteEntry};
use crate::state::{Clients, Config};

pub async fn sync_routes(clients: &Clients, config: &Config) -> ProxyResult<()> {
    let mut filters: HashMap<String, Vec<String>> = HashMap::new();
    filters.insert("label".to_string(), vec!["slasha.managed=true".to_string()]);
    filters.insert("status".to_string(), vec!["running".to_string()]);

    let opts = ListContainersOptionsBuilder::new()
        .all(true)
        .filters(&filters)
        .build();

    let containers = clients.docker.list_containers(Some(opts)).await?;
    let mut routes = Vec::new();

    #[cfg(feature = "bundle")]
    routes.push(RouteEntry {
        domain: config.platform_domain.clone(),
        upstream_host: "host.docker.internal".to_string(),
        upstream_port: config.port,
    });

    for container in containers {
        let Some(labels) = &container.labels else {
            continue;
        };

        if labels.get("slasha.role").map(|v| v.as_str()) == Some("proxy") {
            continue;
        }

        let Some(app_slug) = labels.get("slasha.app_slug") else {
            continue;
        };

        let container_port = match labels
            .get("slasha.container_port")
            .and_then(|p| p.parse::<u16>().ok())
        {
            Some(p) => p,
            None => {
                tracing::warn!("Missing or invalid slasha.container_port for {}", app_slug);
                continue;
            }
        };

        let container_ip = match container
            .network_settings
            .as_ref()
            .and_then(|s| s.networks.as_ref())
            .and_then(|n| n.get(PROXY_NETWORK_NAME))
            .and_then(|net| net.ip_address.as_deref())
            .filter(|ip| !ip.is_empty())
        {
            Some(ip) => ip.to_string(),
            None => {
                tracing::warn!(
                    "Container {} is not attached to the {} network",
                    app_slug,
                    PROXY_NETWORK_NAME
                );
                continue;
            }
        };

        routes.push(RouteEntry {
            domain: format!("{}.{}", app_slug, config.platform_domain),
            upstream_host: container_ip,
            upstream_port: container_port,
        });
    }

    clients.caddy.apply_routes(&routes, config.env).await?;
    tracing::info!("Synced proxy routes: {:#?}", routes);

    Ok(())
}

pub fn spawn_route_syncer(clients: Clients, config: Config) -> Arc<Notify> {
    let notify = Arc::new(Notify::new());

    tokio::spawn({
        let notify = notify.clone();
        async move {
            loop {
                notify.notified().await;
                loop {
                    tokio::select! {
                        _ = sleep(Duration::from_millis(500)) => {
                            if let Err(e) = sync_routes(&clients, &config).await {
                                tracing::error!("Proxy route sync failed: {:?}", e);
                            }
                            break;
                        }
                        _ = notify.notified() => {}
                    }
                }
            }
        }
    });

    notify
}
