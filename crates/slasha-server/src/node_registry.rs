use std::{
    os::unix::fs::PermissionsExt,
    path::PathBuf,
    process::Stdio,
    sync::Arc,
    time::{Duration, Instant},
};

use bollard::Docker;
use dashmap::DashMap;
use slasha_db::models::node::{LOCAL_NODE_ID, Node, NodeStatus};
use tokio::{
    io::{AsyncBufReadExt, AsyncWriteExt, BufReader},
    process::Command,
};

use crate::logs::LogWriter;

/// Cache entry for node connection status and operating system details.
#[derive(Clone, Debug)]
pub struct CachedNodeInfo {
    pub connection_status: String,
    pub os: Option<String>,
    pub last_updated: Instant,
}

/// Registry managing SSH connection credentials, remote execution, Docker API clients, and node status caching.
#[derive(Clone)]
pub struct NodeRegistry {
    nodes_dir: PathBuf,
    keys_dir: PathBuf,
    docker_clients: Arc<DashMap<String, Docker>>,
    status_cache: Arc<DashMap<String, CachedNodeInfo>>,
}

impl NodeRegistry {
    /// Initializes a new [`NodeRegistry`] using the provided directory to store SSH configuration and keys,
    /// and spawns a background health loop to evict dead Docker connections.
    ///
    /// # Arguments
    ///
    /// * `nodes_dir` - Directory path storing node SSH keys and configuration.
    ///
    /// # Returns
    ///
    /// A new [`NodeRegistry`] instance.
    pub fn new(nodes_dir: PathBuf) -> Self {
        let keys_dir = nodes_dir.join("keys");
        let _ = std::fs::create_dir_all(&keys_dir);

        let registry = Self {
            nodes_dir,
            keys_dir,
            docker_clients: Arc::new(DashMap::new()),
            status_cache: Arc::new(DashMap::new()),
        };

        let docker_clients = registry.docker_clients.clone();
        let status_cache = registry.status_cache.clone();

        tokio::spawn(async move {
            loop {
                tokio::time::sleep(Duration::from_secs(30)).await;

                let mut dead_nodes = Vec::new();
                for docker_client in docker_clients.iter() {
                    let node_id = docker_client.key();
                    // do not evict local node client
                    if node_id == LOCAL_NODE_ID {
                        continue;
                    }
                    if let Ok(Err(_)) | Err(_) =
                        tokio::time::timeout(Duration::from_secs(5), docker_client.value().ping())
                            .await
                    {
                        dead_nodes.push(node_id.clone());
                    }
                }

                for node_id in dead_nodes {
                    tracing::warn!(
                        node_id = %node_id,
                        "docker ssh connection died, evicting from registry cache"
                    );
                    docker_clients.remove(&node_id);
                    if let Some(mut entry) = status_cache.get_mut(&node_id) {
                        entry.connection_status = "offline".to_string();
                        entry.os = None;
                        entry.last_updated = Instant::now();
                    }
                }
            }
        });

        registry
    }

    /// Resolves the file path for the SSH `known_hosts` file.
    ///
    /// # Returns
    ///
    /// The absolute path to the `known_hosts` file.
    pub fn known_hosts_path(&self) -> PathBuf {
        self.nodes_dir.join("known_hosts")
    }

    /// Resolves the file path for the SSH `config` file, creating an empty one if it doesn't exist.
    ///
    /// # Returns
    ///
    /// The absolute path to the SSH config file.
    pub fn ssh_config_path(&self) -> anyhow::Result<PathBuf> {
        let path = self.nodes_dir.join("config");
        if !path.exists() {
            std::fs::File::create(&path)?;
        }

        Ok(path)
    }

    /// Resolves and ensures the SSH private key file for the given [`Node`] is present with correct permissions (0600).
    ///
    /// # Arguments
    ///
    /// * `node` - Target remote node ([`Node`]).
    ///
    /// # Returns
    ///
    /// The absolute path to the node's SSH private key file.
    pub fn key_path(&self, node: &Node) -> anyhow::Result<PathBuf> {
        if node.is_local() {
            return Err(anyhow::anyhow!("local node does not use SSH"));
        }

        let key_path = self.keys_dir.join(&node.id);

        if !key_path.exists() {
            let raw_key = node
                .ssh_private_key
                .as_deref()
                .ok_or_else(|| anyhow::anyhow!("node {} has no ssh_private_key", node.id))?;

            let mut normalized = raw_key.replace("\r\n", "\n");
            if !normalized.ends_with('\n') {
                normalized.push('\n');
            }

            std::fs::write(&key_path, normalized)?;
            std::fs::set_permissions(&key_path, std::fs::Permissions::from_mode(0o600))?;
        }

        Ok(key_path)
    }

    /// Constructs `DOCKER_HOST` and `SSH_COMMAND` environment variables for executing `docker` CLI commands over SSH against a remote cluster [`Node`].
    ///
    /// # Arguments
    ///
    /// * `node` - Target remote node ([`Node`]).
    ///
    /// # Returns
    ///
    /// A tuple containing `(DOCKER_HOST, SSH_COMMAND)` environment variable strings.
    pub fn get_docker_ssh_env(&self, node: &Node) -> anyhow::Result<(String, String)> {
        let key_path = self.key_path(node)?;
        let host = node.host.as_deref().unwrap_or("");
        let user = node.user.as_deref().unwrap_or("root");
        let port = node.port.unwrap_or(22);

        let known_hosts_file = self.known_hosts_path();
        let config_file = self.ssh_config_path()?;

        let docker_host = format!("ssh://{user}@{host}:{port}");
        let ssh_cmd = format!(
            "ssh -i {} -p {} -F {} -o UserKnownHostsFile={} -o StrictHostKeyChecking=accept-new -o BatchMode=yes",
            key_path.display(),
            port,
            config_file.display(),
            known_hosts_file.display()
        );

        Ok((docker_host, ssh_cmd))
    }

    /// Verifies the SSH connection to the [`Node`] by running a simple echo command, removing the node's local files on failure.
    ///
    /// # Arguments
    ///
    /// * `node` - Target remote node ([`Node`]).
    pub async fn probe_ssh(&self, node: &Node) -> anyhow::Result<()> {
        let output = self.run_ssh_script(node, "echo ok").await?;

        if output.status.success() {
            Ok(())
        } else {
            self.remove_node_files(node);
            anyhow::bail!(
                "SSH probe failed: {}",
                String::from_utf8_lossy(&output.stderr)
            )
        }
    }

    /// Executes a bash script over SSH on the target [`Node`] and returns the standard output and error.
    ///
    /// # Arguments
    ///
    /// * `node` - Target remote node ([`Node`]).
    /// * `script` - The bash script content to execute.
    ///
    /// # Returns
    ///
    /// The standard [`std::process::Output`] of the executed script.
    pub async fn run_ssh_script(
        &self,
        node: &Node,
        script: &str,
    ) -> anyhow::Result<std::process::Output> {
        let key_path = self.key_path(node)?;
        let host = node.host.as_deref().unwrap_or("");
        let user = node.user.as_deref().unwrap_or("root");
        let port = node.port.unwrap_or(22);

        let known_hosts_file = self.known_hosts_path();
        let config_file = self.ssh_config_path()?;

        let mut child = Command::new("ssh")
            .args([
                "-i",
                key_path.to_str().unwrap_or_default(),
                "-p",
                &port.to_string(),
                "-F",
                config_file.to_str().unwrap_or_default(),
                "-o",
                &format!("UserKnownHostsFile={}", known_hosts_file.to_string_lossy()),
                "-o",
                "StrictHostKeyChecking=accept-new",
                "-o",
                "BatchMode=yes",
                "-o",
                "ConnectTimeout=5",
                &format!("{user}@{host}"),
                "bash",
                "-s",
            ])
            .stdin(Stdio::piped())
            .stdout(Stdio::piped())
            .stderr(Stdio::piped())
            .spawn()?;

        if let Some(mut stdin) = child.stdin.take() {
            stdin.write_all(script.as_bytes()).await?;
        }

        let output = tokio::time::timeout(Duration::from_secs(10), child.wait_with_output())
            .await
            .map_err(|_| anyhow::anyhow!("SSH execution timed out after 10s"))??;

        Ok(output)
    }

    /// Executes a bash script over SSH on the target [`Node`], streaming stdout and stderr to the provided [`LogWriter`].
    ///
    /// # Arguments
    ///
    /// * `node` - Target remote node ([`Node`]).
    /// * `script` - The bash script content to execute.
    /// * `log` - A [`LogWriter`] instance for streaming the output lines.
    ///
    /// # Returns
    ///
    /// A `String` containing the full standard output collected during execution.
    pub async fn run_ssh_script_streaming(
        &self,
        node: &Node,
        script: &str,
        log: &LogWriter,
    ) -> anyhow::Result<String> {
        let key_path = self.key_path(node)?;
        let host = node.host.as_deref().unwrap_or("");
        let user = node.user.as_deref().unwrap_or("root");
        let port = node.port.unwrap_or(22);

        let known_hosts_file = self.known_hosts_path();
        let config_file = self.ssh_config_path()?;

        let mut child = Command::new("ssh")
            .args([
                "-i",
                key_path.to_str().unwrap_or_default(),
                "-p",
                &port.to_string(),
                "-F",
                config_file.to_str().unwrap_or_default(),
                "-o",
                &format!("UserKnownHostsFile={}", known_hosts_file.to_string_lossy()),
                "-o",
                "StrictHostKeyChecking=accept-new",
                "-o",
                "BatchMode=yes",
                "-o",
                "ConnectTimeout=5",
                &format!("{user}@{host}"),
                "bash",
                "-s",
            ])
            .stdin(Stdio::piped())
            .stdout(Stdio::piped())
            .stderr(Stdio::piped())
            .spawn()?;

        if let Some(mut stdin) = child.stdin.take() {
            stdin.write_all(script.as_bytes()).await?;
        }

        let mut stdout_buffer = String::new();

        let stdout = child.stdout.take().map(BufReader::new);
        let stderr = child.stderr.take().map(BufReader::new);

        let drain_stdout = async {
            if let Some(reader) = stdout {
                let mut lines = reader.lines();
                while let Some(line) = lines.next_line().await? {
                    stdout_buffer.push_str(&line);
                    stdout_buffer.push('\n');
                    log.stdout(line);
                }
            }
            Ok::<(), anyhow::Error>(())
        };

        let drain_stderr = async {
            if let Some(reader) = stderr {
                let mut lines = reader.lines();
                while let Some(line) = lines.next_line().await? {
                    log.stderr(line);
                }
            }
            Ok::<(), anyhow::Error>(())
        };

        tokio::try_join!(drain_stdout, drain_stderr)?;

        let status = child.wait().await?;
        if !status.success() {
            anyhow::bail!("SSH script failed with exit status {}", status);
        }

        Ok(stdout_buffer)
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

    /// Obtains or establishes a cached Docker API client connection for a node.
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
            let key_path = self.key_path(node)?;
            let known_hosts_file = self.known_hosts_path();
            let config_file = self.ssh_config_path()?;

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
                .with_connect_timeout(Duration::from_secs(10))
                .with_known_hosts_check(bollard::KnownHosts::Add);

            Docker::connect_with_ssh_options(&address, 120, bollard::API_DEFAULT_VERSION, options)?
        };

        self.docker_clients.insert(node.id.clone(), docker.clone());

        Ok(docker)
    }

    /// Resolves connection status and OS details for a node, serving cached results if valid.
    ///
    /// # Arguments
    ///
    /// * `node` - Target node model ([`Node`]).
    ///
    /// # Returns
    ///
    /// A tuple containing `(connection_status, os)`.
    pub async fn resolve_node_info(&self, node: &Node) -> (String, Option<String>) {
        if !matches!(node.status, NodeStatus::Ready) {
            return ("offline".to_string(), None);
        }

        if let Some(entry) = self.status_cache.get(&node.id)
            && entry.last_updated.elapsed() < Duration::from_secs(15)
        {
            return (entry.connection_status.clone(), entry.os.clone());
        }

        let (status, os) = match self.get_client(node) {
            Ok(client) => match client.info().await {
                Ok(info) => ("online".to_string(), info.operating_system),
                Err(_) => {
                    self.docker_clients.remove(&node.id);
                    ("offline".to_string(), None)
                }
            },
            Err(_) => ("offline".to_string(), None),
        };

        self.status_cache.insert(
            node.id.clone(),
            CachedNodeInfo {
                connection_status: status.clone(),
                os: os.clone(),
                last_updated: Instant::now(),
            },
        );

        (status, os)
    }

    /// Deletes SSH key and known host entries from disk for a node.
    ///
    /// # Arguments
    ///
    /// * `node` - Target node model ([`Node`]).
    fn remove_node_files(&self, node: &Node) {
        let _ = std::fs::remove_file(self.keys_dir.join(&node.id));

        if let Some(host) = &node.host {
            let known_hosts = self.known_hosts_path();
            if known_hosts.exists() {
                let _ = std::process::Command::new("ssh-keygen")
                    .args(["-f", known_hosts.to_str().unwrap_or_default(), "-R", host])
                    .output();

                if let Some(port) = node.port
                    && port != 22
                {
                    let _ = std::process::Command::new("ssh-keygen")
                        .args([
                            "-f",
                            known_hosts.to_str().unwrap_or_default(),
                            "-R",
                            &format!("[{}]:{}", host, port),
                        ])
                        .output();
                }
            }
        }
    }

    /// Evicts a node's cached Docker client connection, clears its status cache, and removes SSH key artifacts.
    ///
    /// # Arguments
    ///
    /// * `node` - Target node model ([`Node`]).
    pub fn remove(&self, node: &Node) {
        self.docker_clients.remove(&node.id);
        self.status_cache.remove(&node.id);
        self.remove_node_files(node);
    }
}
