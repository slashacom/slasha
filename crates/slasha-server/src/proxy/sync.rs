use std::{collections::HashMap, sync::Arc};

use bollard::query_parameters::ListContainersOptionsBuilder;
use slasha_db::{DbPool, repos::app_domain::AppDomainRepo};
use tokio::{
    sync::Notify,
    time::{Duration, sleep},
};

use super::{PROXY_NETWORK_NAME, ProxyResult, RouteEntry, Upstream};
use crate::state::{Clients, Config};

pub async fn sync_routes(clients: &Clients, db_pool: &DbPool, config: &Config) -> ProxyResult<()> {
    let mut filters: HashMap<String, Vec<String>> = HashMap::new();
    filters.insert("label".to_string(), vec!["slasha.managed=true".to_string()]);
    filters.insert("status".to_string(), vec!["running".to_string()]);

    let opts = ListContainersOptionsBuilder::new()
        .all(true)
        .filters(&filters)
        .build();

    let containers = clients.docker.list_containers(Some(opts)).await?;
    let mut domain_upstreams: HashMap<String, Vec<Upstream>> = HashMap::new();

    #[cfg(feature = "bundle")]
    domain_upstreams.insert(
        config.platform_domain.clone(),
        vec![Upstream {
            host: "host.docker.internal".to_string(),
            port: config.port,
        }],
    );

    for container in containers {
        let Some(labels) = &container.labels else {
            continue;
        };

        if labels.get("slasha.role").map(|v| v.as_str()) == Some("proxy") {
            continue;
        }

        let Some(app_id) = labels.get("slasha.app_id") else {
            continue;
        };

        let Some(app_slug) = labels.get("slasha.app_slug") else {
            continue;
        };

        if labels.get("slasha.process_type").map(|v| v.as_str()) != Some("web") {
            continue;
        }

        let container_port = match labels
            .get("slasha.container_port")
            .and_then(|p| p.parse::<u16>().ok())
        {
            Some(p) => p,
            None => {
                tracing::warn!(
                    app_slug = %app_slug,
                    "Missing or invalid slasha.container_port"
                );
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
                    app_slug = %app_slug,
                    network = %PROXY_NETWORK_NAME,
                    "Container is not attached to the network"
                );
                continue;
            }
        };

        let upstream = Upstream {
            host: container_ip,
            port: container_port,
        };

        // add default domain
        let default_domain = format!("{}.{}", app_slug, config.platform_domain);
        domain_upstreams
            .entry(default_domain)
            .or_default()
            .push(upstream.clone());

        // add custom domains
        let custom_domains = AppDomainRepo::list_for_app(db_pool, app_id).await?;
        for domain in custom_domains {
            domain_upstreams
                .entry(domain.domain)
                .or_default()
                .push(upstream.clone());
        }
    }

    let routes: Vec<RouteEntry> = domain_upstreams
        .into_iter()
        .map(|(domain, upstreams)| RouteEntry { domain, upstreams })
        .collect();

    clients.caddy.apply_routes(&routes, config.env).await?;
    tracing::debug!(routes = ?routes, "synced proxy routes");

    Ok(())
}

pub fn spawn_route_syncer(clients: Clients, db_pool: DbPool, config: Config) -> Arc<Notify> {
    let notify = Arc::new(Notify::new());

    tokio::spawn({
        let notify = notify.clone();
        async move {
            loop {
                notify.notified().await;
                loop {
                    tokio::select! {
                        _ = sleep(Duration::from_millis(500)) => {
                            if let Err(e) = sync_routes(&clients, &db_pool, &config).await {
                                tracing::error!(
                                    error = ?e,
                                    "Proxy route sync failed"
                                );
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
