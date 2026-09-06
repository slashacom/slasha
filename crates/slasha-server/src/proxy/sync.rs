use std::{collections::HashMap, sync::Arc};

use bollard::{models::ContainerSummary, query_parameters::ListContainersOptionsBuilder};
use slasha_db::{
    DbPool,
    deployment::DeploymentStatus,
    models::node::Node,
    repos::{app_domain::AppDomainRepo, deployment::DeploymentRepo, node::NodeRepo},
};
use tokio::{
    sync::Notify,
    time::{Duration, sleep},
};

use super::{
    CaddyClient, PROXY_NETWORK_NAME, ProxyError, RouteEntry, Upstream, error::ProxyResult,
};
use crate::{
    docker::labels::{
        LABEL_APP_ID, LABEL_APP_SLUG, LABEL_CONTAINER_PORT, LABEL_DEPLOYMENT_ID, LABEL_MANAGED,
        LABEL_PROCESS_TYPE, LABEL_ROLE,
    },
    node_registry::NodeRegistry,
    state::{Clients, Config},
};

async fn apply_remote_routes_via_ssh(
    node_registry: &NodeRegistry,
    node: &Node,
    routes: &[RouteEntry],
    self_signed_domains: &[String],
    config: &Config,
) -> ProxyResult<()> {
    let caddy_config = CaddyClient::build_routes_config(routes, self_signed_domains, config.env);

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

    let output = node_registry
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

/// Represents routing and upstream information parsed from a container.
struct ContainerInfo {
    app_id: String,
    app_slug: String,
    deployment_id: String,
    upstream: Upstream,
}

/// Extracts routing information and upstream details from a container's labels and network settings.
///
/// # Arguments
///
/// * `container` - Container summary returned by Docker ([`ContainerSummary`]).
///
/// # Returns
///
/// An [`Option<WebContainerInfo>`] containing the parsed details.
fn extract_container_info(container: &ContainerSummary) -> Option<ContainerInfo> {
    let labels = container.labels.as_ref()?;

    if labels.get(LABEL_ROLE).map(|v| v.as_str()) == Some("proxy") {
        return None;
    }

    if labels.get(LABEL_PROCESS_TYPE).map(|v| v.as_str()) != Some("web") {
        return None;
    }

    let app_id = labels.get(LABEL_APP_ID)?.clone();
    let app_slug = labels.get(LABEL_APP_SLUG)?.clone();
    let deployment_id = labels.get(LABEL_DEPLOYMENT_ID)?.clone();

    let container_port = match labels
        .get(LABEL_CONTAINER_PORT)
        .and_then(|p| p.parse::<u16>().ok())
    {
        Some(p) => p,
        None => {
            tracing::warn!(
                app_slug = %app_slug,
                "missing or invalid container port label"
            );
            return None;
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
                "container is not attached to the proxy network"
            );
            return None;
        }
    };

    Some(ContainerInfo {
        app_id,
        app_slug,
        deployment_id,
        upstream: Upstream {
            host: container_ip,
            port: container_port,
        },
    })
}

/// Synchronizes routing configuration for all active deployments across all nodes.
///
/// # Arguments
///
/// * `clients` - Application clients including Docker and Caddy ([`Clients`]).
/// * `db_pool` - Database connection pool ([`DbPool`]).
/// * `config` - Application configuration ([`Config`]).
///
/// # Returns
///
/// A [`ProxyResult`] indicating success or failure.
pub async fn sync_routes(
    node_registry: &NodeRegistry,
    clients: &Clients,
    db_pool: &DbPool,
    config: &Config,
) -> ProxyResult<()> {
    let nodes = NodeRepo::list(db_pool).await?;

    let mut local_routes: Vec<RouteEntry> = Vec::new();

    #[cfg(feature = "bundle")]
    local_routes.push(RouteEntry {
        domain: config.platform_domain.clone(),
        upstreams: vec![Upstream {
            host: "host.docker.internal".to_string(),
            port: config.port,
        }],
        tls_root_ca: None,
        tls_server_name: None,
    });

    let mut filters: HashMap<String, Vec<String>> = HashMap::new();
    filters.insert("label".to_string(), vec![format!("{}=true", LABEL_MANAGED)]);
    filters.insert("status".to_string(), vec!["running".to_string()]);

    let list_container_opts = ListContainersOptionsBuilder::new()
        .all(true)
        .filters(&filters)
        .build();

    for node in nodes {
        let is_local = node.is_local();
        let docker_client = match node_registry.get_client(&node) {
            Ok(c) => c,
            Err(e) => {
                tracing::warn!(node_id = %node.id, error = %e, "failed to get docker client for node, skipping route sync");
                continue;
            }
        };

        let containers: Vec<ContainerInfo> = match docker_client
            .list_containers(Some(list_container_opts.clone()))
            .await
        {
            Ok(c) => c.iter().filter_map(extract_container_info).collect(),
            Err(e) => {
                tracing::warn!(node_id = %node.id, error = %e, "failed to list containers for node, skipping route sync");
                continue;
            }
        };

        let deployment_ids: Vec<String> = containers
            .iter()
            .map(|info| info.deployment_id.clone())
            .collect();

        let deployment_status_map: HashMap<String, DeploymentStatus> =
            DeploymentRepo::find_by_ids(db_pool, deployment_ids)
                .await?
                .into_iter()
                .map(|d| (d.id, d.status))
                .collect();

        // (app_id, app_slug) -> running upstreams
        let mut app_upstreams: HashMap<(String, String), Vec<Upstream>> = HashMap::new();

        for info in containers {
            match deployment_status_map.get(&info.deployment_id) {
                Some(DeploymentStatus::Running) => {
                    app_upstreams
                        .entry((info.app_id, info.app_slug))
                        .or_default()
                        .push(info.upstream);
                }
                Some(_) => {
                    tracing::warn!(
                        app_slug = %info.app_slug,
                        deployment_id = %info.deployment_id,
                        "deployment is not in running state, skipping route"
                    );
                }
                None => {
                    tracing::warn!(
                        app_slug = %info.app_slug,
                        deployment_id = %info.deployment_id,
                        "container has no deployment record, skipping route"
                    );
                }
            }
        }

        let app_ids: Vec<String> = app_upstreams
            .keys()
            .map(|(app_id, _)| app_id.clone())
            .collect();

        let custom_domains = AppDomainRepo::list_for_apps(db_pool, app_ids).await?;
        let mut domains_by_app: HashMap<String, Vec<String>> = HashMap::new();
        for domain in custom_domains {
            domains_by_app
                .entry(domain.app_id)
                .or_default()
                .push(domain.domain);
        }

        if is_local {
            for ((app_id, app_slug), upstreams) in app_upstreams {
                let default_domain = format!("{}.{}", app_slug, config.platform_domain);
                let custom_domains = domains_by_app.remove(&app_id).unwrap_or_default();
                let all_domains = std::iter::once(default_domain).chain(custom_domains);

                for domain in all_domains {
                    local_routes.push(RouteEntry {
                        domain,
                        upstreams: upstreams.clone(),
                        tls_root_ca: None,
                        tls_server_name: None,
                    });
                }
            }
        } else {
            let Some(node_host) = &node.host else {
                tracing::warn!(node_id = %node.id, "remote node has no host configured, skipping route sync");
                continue;
            };

            let mut node_routes: Vec<RouteEntry> = Vec::new();

            for ((app_id, app_slug), upstreams) in app_upstreams {
                let default_domain = format!("{}.{}", app_slug, config.platform_domain);
                let custom_domains = domains_by_app.remove(&app_id).unwrap_or_default();
                let all_domains = std::iter::once(default_domain.clone()).chain(custom_domains);

                for domain in all_domains {
                    node_routes.push(RouteEntry {
                        domain: domain.clone(),
                        upstreams: upstreams.clone(),
                        tls_root_ca: None,
                        tls_server_name: None,
                    });

                    local_routes.push(RouteEntry {
                        domain,
                        upstreams: vec![Upstream {
                            host: node_host.clone(),
                            port: 443,
                        }],
                        // proxy securely to remote nodes by trusting their internal ca
                        // override sni to default domain so remote node presents its self-signed cert,
                        // even when routing a custom domain.
                        tls_root_ca: node.internal_root_ca.clone(),
                        tls_server_name: Some(default_domain.clone()),
                    });
                }
            }

            node_routes.sort_by(|a, b| a.domain.cmp(&b.domain));

            let self_signed_domains = vec![format!("*.{}", config.platform_domain)];

            if let Err(e) = apply_remote_routes_via_ssh(
                node_registry,
                &node,
                &node_routes,
                &self_signed_domains,
                config,
            )
            .await
            {
                tracing::error!(
                    node_id = %node.id,
                    error = %e,
                    "failed to sync routes to remote node"
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

    local_routes.sort_by(|a, b| a.domain.cmp(&b.domain));

    let self_signed_domains = vec![];
    clients
        .caddy_client
        .apply_routes(
            &local_routes,
            &self_signed_domains,
            config.env,
            "http://127.0.0.1:2019",
        )
        .await?;

    tracing::debug!(routes = ?local_routes, "synced proxy routes for local server");

    Ok(())
}

/// Spawns a background task that synchronizes proxy routes whenever notified.
///
/// # Arguments
///
/// * `clients` - Application clients including Docker and Caddy ([`Clients`]).
/// * `db_pool` - Database connection pool ([`DbPool`]).
/// * `config` - Application configuration ([`Config`]).
///
/// # Returns
///
/// An [`Arc<Notify>`] used to trigger route synchronization.
pub fn spawn_route_syncer(
    node_registry: NodeRegistry,
    clients: Clients,
    db_pool: DbPool,
    config: Config,
) -> Arc<Notify> {
    let notify = Arc::new(Notify::new());

    tokio::spawn({
        let notify = notify.clone();
        async move {
            loop {
                notify.notified().await;
                loop {
                    tokio::select! {
                        _ = sleep(Duration::from_millis(500)) => {
                            if let Err(e) = sync_routes(&node_registry, &clients, &db_pool, &config).await {
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
