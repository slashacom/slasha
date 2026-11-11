use std::{collections::HashMap, sync::Arc};

use bollard::query_parameters::EventsOptionsBuilder;
use dashmap::DashSet;
use futures_util::StreamExt;
use slasha_db::{DbPool, models::node::Node, repos::node::NodeRepo};
use tokio::{
    sync::Notify,
    time::{Duration, sleep},
};

use crate::{
    docker::labels::{LABEL_MANAGED, LABEL_PROCESS_TYPE},
    node_registry::NodeRegistry,
};

const NODE_SCAN_INTERVAL: Duration = Duration::from_secs(30);
const RECONNECT_DELAY: Duration = Duration::from_secs(5);

/// Keeps proxy routes in step with containers Docker restarts on its own.
///
/// A web container runs under `unless-stopped`, so when its process crashes
/// Docker brings it back without asking us, and it may come back on a
/// different IP. Routes are only rebuilt when Slasha itself acts, so without
/// this the proxy keeps dialling the old address until the next deploy. Every
/// node's event stream is watched for a managed web container starting, and
/// each (re)connect syncs once as well, since a start missed while the stream
/// was down would otherwise never be seen.
///
/// # Arguments
///
/// * `node_registry` - Docker clients per node ([`NodeRegistry`]).
/// * `db_pool` - Database connection pool ([`DbPool`]).
/// * `proxy_sync_trigger` - Trigger for the route syncer ([`Notify`]).
pub fn spawn_container_watcher(
    node_registry: NodeRegistry,
    db_pool: DbPool,
    proxy_sync_trigger: Arc<Notify>,
) {
    let watching: Arc<DashSet<String>> = Arc::new(DashSet::new());

    tokio::spawn(async move {
        loop {
            match NodeRepo::list(&db_pool).await {
                Ok(nodes) => {
                    for node in nodes {
                        if !watching.insert(node.id.clone()) {
                            continue;
                        }

                        tokio::spawn(watch_node(
                            node,
                            node_registry.clone(),
                            db_pool.clone(),
                            proxy_sync_trigger.clone(),
                            watching.clone(),
                        ));
                    }
                }
                Err(e) => {
                    tracing::warn!(error = %e, "failed to list nodes for container watcher");
                }
            }

            sleep(NODE_SCAN_INTERVAL).await;
        }
    });
}

async fn watch_node(
    node: Node,
    node_registry: NodeRegistry,
    db_pool: DbPool,
    proxy_sync_trigger: Arc<Notify>,
    watching: Arc<DashSet<String>>,
) {
    let mut filters: HashMap<&str, Vec<String>> = HashMap::new();
    filters.insert("type", vec!["container".to_string()]);
    filters.insert("event", vec!["start".to_string()]);
    filters.insert(
        "label",
        vec![
            format!("{}=true", LABEL_MANAGED),
            format!("{}=web", LABEL_PROCESS_TYPE),
        ],
    );

    loop {
        let docker = match node_registry.get_client(&node) {
            Ok(d) => d,
            Err(e) => {
                tracing::warn!(node_id = %node.id, error = %e, "no docker client for container watcher");
                break;
            }
        };

        let options = EventsOptionsBuilder::new().filters(&filters).build();
        let mut events = docker.events(Some(options));

        proxy_sync_trigger.notify_one();

        while let Some(event) = events.next().await {
            match event {
                Ok(_) => proxy_sync_trigger.notify_one(),
                Err(e) => {
                    tracing::warn!(node_id = %node.id, error = %e, "container event stream failed");
                    break;
                }
            }
        }

        sleep(RECONNECT_DELAY).await;

        if NodeRepo::get(&db_pool, &node.id).await.is_err() {
            break;
        }
    }

    watching.remove(&node.id);
}
