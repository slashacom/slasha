use std::{os::unix::fs::PermissionsExt, path::PathBuf, process::Stdio};

use slasha_db::models::node::Node;
use tokio::{
    io::{AsyncBufReadExt, AsyncWriteExt, BufReader},
    process::Command,
};

use crate::logs::LogWriter;

/// Manages SSH connections, key files, and known hosts configurations for remote nodes.
#[derive(Clone)]
pub struct NodeConnectionManager {
    nodes_dir: PathBuf,
    keys_dir: PathBuf,
}

impl NodeConnectionManager {
    /// Initializes a new [`NodeConnectionManager`] using the provided directory to store SSH configuration and keys.
    ///
    /// # Arguments
    ///
    /// * `nodes_dir` - Path to the directory where SSH artifacts will be stored.
    pub fn new(nodes_dir: PathBuf) -> Self {
        let keys_dir = nodes_dir.join("keys");
        let _ = std::fs::create_dir_all(&keys_dir);

        Self {
            nodes_dir,
            keys_dir,
        }
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
    /// * `node` - Target remote cluster node ([`Node`]).
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

            // normalize line endings to Unix (LF) and ensure a trailing newline is present
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
    /// * `node` - Target remote cluster node ([`Node`]).
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
    /// * `node` - Target remote cluster node ([`Node`]).
    pub async fn probe_ssh(&self, node: &Node) -> anyhow::Result<()> {
        let output = self.run_ssh_script(node, "echo ok").await?;

        if output.status.success() {
            Ok(())
        } else {
            self.remove_node(node);
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
    /// * `node` - Target remote cluster node ([`Node`]).
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

        let output =
            tokio::time::timeout(std::time::Duration::from_secs(10), child.wait_with_output())
                .await
                .map_err(|_| anyhow::anyhow!("SSH execution timed out after 10s"))??;

        Ok(output)
    }

    /// Executes a bash script over SSH on the target [`Node`], streaming stdout and stderr to the provided [`LogWriter`].
    ///
    /// # Arguments
    ///
    /// * `node` - Target remote cluster node ([`Node`]).
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

    /// Deletes local SSH artifacts (private key and `known_hosts` entries) associated with the given [`Node`].
    ///
    /// # Arguments
    ///
    /// * `node` - Target remote cluster node ([`Node`]).
    pub fn remove_node(&self, node: &Node) {
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
}
