use std::sync::Arc;

use bollard::Docker;
use dashmap::DashMap;
use slasha_db::models::node::{LOCAL_NODE_ID, Node};

use crate::node_connection_manager::NodeConnectionManager;

/// Registry managing cached Docker API client connections across local and remote SSH nodes.
#[derive(Clone)]
pub struct DockerRegistry {
    node_connection_manager: Arc<NodeConnectionManager>,
    docker_clients: Arc<DashMap<String, Docker>>, // node_id -> docker client
}

impl DockerRegistry {
    /// Creates a new [`DockerRegistry`] and spawns a background task to evict dead SSH node connections.
    ///
    /// # Arguments
    ///
    /// * `node_connection_manager` - Node connection manager handle ([`NodeConnectionManager`]).
    ///
    /// # Returns
    ///
    /// A new [`DockerRegistry`] instance.
    pub fn new(node_connection_manager: Arc<NodeConnectionManager>) -> Self {
        let registry = Self {
            node_connection_manager,
            docker_clients: Arc::new(DashMap::new()),
        };

        let clients = registry.docker_clients.clone();
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
                        entry.value().ping(),
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

    /// Returns a Docker API client connected via local socket defaults.
    ///
    /// # Returns
    ///
    /// An [`anyhow::Result`] containing the [`Docker`] client instance.
    pub fn get_local_client(&self) -> anyhow::Result<Docker> {
        if let Some(entry) = self.docker_clients.get(LOCAL_NODE_ID) {
            return Ok(entry.clone());
        }

        let docker = Docker::connect_with_local_defaults()?;

        self.docker_clients
            .insert(LOCAL_NODE_ID.to_string(), docker.clone());

        Ok(docker)
    }

    /// Obtains or establishes a cached Docker API client connection for a cluster node.
    ///
    /// # Arguments
    ///
    /// * `node` - Target node model ([`Node`]).
    ///
    /// # Returns
    ///
    /// An [`anyhow::Result`] containing the [`Docker`] client instance.
    pub fn get_client(&self, node: &Node) -> anyhow::Result<Docker> {
        if let Some(entry) = self.docker_clients.get(&node.id) {
            return Ok(entry.clone());
        }

        let docker = if node.is_local() {
            Docker::connect_with_local_defaults()?
        } else {
            let key_path = self.node_connection_manager.key_path(node)?;
            let known_hosts_file = self.node_connection_manager.known_hosts_path();
            let config_file = self.node_connection_manager.ssh_config_path()?;

            let address = format!(
                "ssh://{}@{}:{}",
                node.user.as_deref().unwrap_or("root"),
                node.host.as_deref().unwrap_or(""),
                node.port.unwrap_or(22)
            );

            let options = bollard::SshOptions::new()
                .with_keypair_path(key_path.to_string_lossy().to_string())
                .with_user_known_hosts_file(known_hosts_file.to_string_lossy().to_string())
                .with_config_file(config_file.to_string_lossy().to_string())
                .with_connect_timeout(std::time::Duration::from_secs(10))
                .with_known_hosts_check(bollard::KnownHosts::Add);

            Docker::connect_with_ssh_options(&address, 120, bollard::API_DEFAULT_VERSION, options)?
        };

        self.docker_clients.insert(node.id.clone(), docker.clone());

        Ok(docker)
    }

    /// Evicts a node's cached Docker client connection and removes its stored SSH keys and known hosts.
    ///
    /// # Arguments
    ///
    /// * `node` - Target node model.
    pub fn remove(&self, node: &Node) {
        self.docker_clients.remove(&node.id);
        self.node_connection_manager.remove_node(node);
    }
}
