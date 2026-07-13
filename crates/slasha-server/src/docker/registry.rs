use std::sync::Arc;

use bollard::Docker;
use dashmap::DashMap;
use slasha_db::models::node::{LOCAL_NODE_ID, Node};

use crate::node_connection_manager::NodeConnectionManager;

struct NodeClient {
    docker: Docker,
}

#[derive(Clone)]
pub struct DockerRegistry {
    node_connection_manager: Arc<NodeConnectionManager>,
    clients: Arc<DashMap<String, Arc<NodeClient>>>,
}

impl DockerRegistry {
    pub fn new(node_connection_manager: Arc<NodeConnectionManager>) -> Self {
        let registry = Self {
            node_connection_manager,
            clients: Arc::new(DashMap::new()),
        };

        let clients = registry.clients.clone();
        tokio::spawn(async move {
            loop {
                tokio::time::sleep(std::time::Duration::from_secs(30)).await;

                let mut dead_nodes = Vec::new();
                for entry in clients.iter() {
                    let node_id = entry.key();
                    if node_id == LOCAL_NODE_ID {
                        continue; // don't evict the local node's connection
                    }
                    if let Ok(Err(_)) | Err(_) = tokio::time::timeout(
                        std::time::Duration::from_secs(5),
                        entry.value().docker.ping(),
                    )
                    .await
                    {
                        dead_nodes.push(node_id.clone());
                    }
                }

                for node_id in dead_nodes {
                    tracing::warn!(
                        node_id = %node_id,
                        "docker ssh connection died, evicting from cache"
                    );
                    clients.remove(&node_id);
                }
            }
        });

        registry
    }

    pub fn get_local_client(&self) -> anyhow::Result<Docker> {
        if let Some(entry) = self.clients.get(LOCAL_NODE_ID) {
            return Ok(entry.docker.clone());
        }

        let docker = Docker::connect_with_local_defaults()?;

        self.clients.insert(
            LOCAL_NODE_ID.to_string(),
            Arc::new(NodeClient {
                docker: docker.clone(),
            }),
        );

        Ok(docker)
    }

    pub fn get_client(&self, node: &Node) -> anyhow::Result<Docker> {
        if let Some(entry) = self.clients.get(&node.id) {
            return Ok(entry.docker.clone());
        }

        let docker = if node.is_local() {
            Docker::connect_with_local_defaults()?
        } else {
            let key_path = self.node_connection_manager.get_key_path(node)?;
            let known_hosts_file = self.node_connection_manager.known_hosts_path();
            let config_file = self.node_connection_manager.ssh_config_path()?;

            let address = format!(
                "ssh://{}@{}:{}",
                node.user.as_deref().unwrap_or("root"),
                node.host.as_deref().unwrap_or(""),
                node.port.unwrap_or(22)
            );

            let options = bollard::SshOptions {
                keypair_path: Some(key_path.to_string_lossy().to_string()),
                user_known_hosts_file: Some(known_hosts_file.to_string_lossy().to_string()),
                config_file: Some(config_file.to_string_lossy().to_string()),
                connect_timeout: Some(std::time::Duration::from_secs(10)),
                known_hosts_check: Some(bollard::KnownHosts::Add),
            };

            Docker::connect_with_ssh_options(
                &address,
                120,
                bollard::API_DEFAULT_VERSION,
                options,
            )?
        };

        self.clients.insert(
            node.id.clone(),
            Arc::new(NodeClient {
                docker: docker.clone(),
            }),
        );

        Ok(docker)
    }

    pub fn remove(&self, node_id: &str) {
        self.clients.remove(node_id);
        self.node_connection_manager.remove_key(node_id);
    }
}
