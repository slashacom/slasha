use std::{collections::HashMap, sync::Arc};

use bollard::query_parameters::ListContainersOptionsBuilder;
use slasha_db::{
    DbError, DbPool,
    deployment::DeploymentStatus,
    repos::{app_domain::AppDomainRepo, deployment::DeploymentRepo, node::NodeRepo},
};
use tokio::{
    sync::Notify,
    time::{Duration, sleep},
};

use super::{PROXY_NETWORK_NAME, ProxyError, ProxyResult, RouteEntry, Upstream};
use crate::state::{Clients, Config};

async fn apply_remote_routes_via_ssh(
    clients: &Clients,
    node: &slasha_db::models::node::Node,
    routes: &[RouteEntry],
    internal_domains: &[String],
    config: &Config,
) -> ProxyResult<()> {
    let caddy_config =
        clients
            .caddy_client
            .build_routes_config(routes, internal_domains, config.env);

    let caddy_config = serde_json::to_string(&caddy_config)
        .map_err(|e| ProxyError::Caddy(format!("failed to serialize caddy config: {e}")))?;

    let script = format!(
        r#"set -euo pipefail
status=$(curl -sS -o /dev/stderr -w '%{{http_code}}' \
  -X POST \
  -H 'Content-Type: application/json' \
  --data-binary @- \
  http://127.0.0.1:2019/load <<'SLASHA_CADDY_CONFIG'
{}
SLASHA_CADDY_CONFIG
) || true
if [ "${{status:-0}}" -lt 200 ] || [ "${{status:-0}}" -ge 300 ]; then
  exit 1
fi
"#,
        caddy_config
    );

    let output = clients
        .node_connection_manager
        .run_ssh_script(node, &script)
        .await
        .map_err(|e| ProxyError::Caddy(format!("remote caddy ssh failed: {e}")))?;

    if !output.status.success() {
        return Err(ProxyError::Caddy(format!(
            "remote caddy apply failed: {}",
            String::from_utf8_lossy(&output.stderr)
        )));
    }

    Ok(())
}

pub async fn sync_routes(clients: &Clients, db_pool: &DbPool, config: &Config) -> ProxyResult<()> {
    let mut filters: HashMap<String, Vec<String>> = HashMap::new();
    filters.insert("label".to_string(), vec!["slasha.managed=true".to_string()]);
    filters.insert("status".to_string(), vec!["running".to_string()]);

    let opts = ListContainersOptionsBuilder::new()
        .all(true)
        .filters(&filters)
        .build();

    let nodes = NodeRepo::list(db_pool).await?;

    // domain -> (list of upstreams, tls_root_ca, tls_server_name)
    let mut local_server_upstreams: HashMap<
        String,
        (Vec<Upstream>, Option<String>, Option<String>),
    > = HashMap::new();

    #[cfg(feature = "bundle")]
    local_server_upstreams.insert(
        config.platform_domain.clone(),
        (
            vec![Upstream {
                host: "host.docker.internal".to_string(),
                port: config.port,
            }],
            None,
            None,
        ),
    );

    for node in nodes {
        let is_local = node.is_local();
        let docker_client = match clients.docker_registry.get_client(&node) {
            Ok(c) => c,
            Err(e) => {
                tracing::warn!(node_id = %node.id, error = %e, "Failed to get docker client for node, skipping route sync");
                continue;
            }
        };

        let containers = match docker_client.list_containers(Some(opts.clone())).await {
            Ok(c) => c,
            Err(e) => {
                tracing::warn!(node_id = %node.id, error = %e, "Failed to list containers for node, skipping route sync");
                continue;
            }
        };

        // domain -> list of upstreams
        let mut node_domain_upstreams: HashMap<String, Vec<Upstream>> = HashMap::new();

        for container in containers {
            let Some(labels) = &container.labels else {
                continue;
            };

            // skip the caddy container itself
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

            let Some(deployment_id) = labels.get("slasha.deployment_id") else {
                continue;
            };

            // we only want to route for running deployments
            match DeploymentRepo::find(db_pool, deployment_id, app_id).await {
                Ok(deployment) => {
                    if deployment.status != DeploymentStatus::Running {
                        continue;
                    }
                }
                Err(DbError::NotFound(_)) => {
                    tracing::warn!(
                        app_slug = %app_slug,
                        deployment_id = %deployment_id,
                        "Container has no deployment record, skipping route"
                    );
                    continue;
                }
                Err(e) => return Err(e.into()),
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
            node_domain_upstreams
                .entry(default_domain.clone())
                .or_default()
                .push(upstream.clone());

            // add custom domains
            let custom_domains = AppDomainRepo::list_for_app(db_pool, app_id).await?;
            for domain in &custom_domains {
                node_domain_upstreams
                    .entry(domain.domain.clone())
                    .or_default()
                    .push(upstream.clone());
            }

            // populate main server fallback routes
            let local_server_upstream = if is_local {
                upstream
            } else {
                Upstream {
                    host: node.host.clone().unwrap(),
                    port: 443,
                }
            };

            local_server_upstreams
                .entry(default_domain.clone())
                .or_default()
                .0
                .push(local_server_upstream.clone());

            if !is_local {
                let entry = local_server_upstreams.get_mut(&default_domain).unwrap();
                entry.1 = node.internal_root_ca.clone();
                entry.2 = Some(default_domain.clone());
            }

            for domain in &custom_domains {
                local_server_upstreams
                    .entry(domain.domain.clone())
                    .or_default()
                    .0
                    .push(local_server_upstream.clone());

                if !is_local {
                    let entry = local_server_upstreams.get_mut(&domain.domain).unwrap();
                    entry.1 = node.internal_root_ca.clone();
                    entry.2 = Some(default_domain.clone());
                }
            }
        }

        if !is_local {
            let node_routes: Vec<RouteEntry> = node_domain_upstreams
                .into_iter()
                .map(|(domain, upstreams)| RouteEntry {
                    domain,
                    upstreams,
                    tls_root_ca: None,
                    tls_server_name: None,
                })
                .collect();

            let internal_domains = vec![format!("*.{}", config.platform_domain)];

            if let Err(e) =
                apply_remote_routes_via_ssh(clients, &node, &node_routes, &internal_domains, config)
                    .await
            {
                tracing::error!(
                    node_id = %node.id,
                    error = %e,
                    "Failed to sync routes to remote node"
                );
            } else {
                tracing::debug!(
                    node_id = %node.id,
                    routes = ?node_routes,
                    "synced proxy routes for remote node"
                );
            }
        }
    }

    let local_routes: Vec<RouteEntry> = local_server_upstreams
        .into_iter()
        .map(
            |(domain, (upstreams, tls_root_ca, tls_server_name))| RouteEntry {
                domain,
                upstreams,
                tls_root_ca,
                tls_server_name,
            },
        )
        .collect();

    let local_internal_domains = vec![];
    clients
        .caddy_client
        .apply_routes(
            &local_routes,
            &local_internal_domains,
            config.env,
            "http://127.0.0.1:2019",
        )
        .await?;

    tracing::debug!(routes = ?local_routes, "synced proxy routes for main server");

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
