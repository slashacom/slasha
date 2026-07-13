use std::{os::unix::fs::PermissionsExt, path::PathBuf, process::Stdio};

use slasha_db::models::node::Node;
use tokio::{
    io::{AsyncBufReadExt, AsyncWriteExt, BufReader},
    process::Command,
};

use crate::logs::LogHandle;

#[derive(Clone)]
pub struct NodeConnectionManager {
    nodes_dir: PathBuf,
    keys_dir: PathBuf,
}

impl NodeConnectionManager {
    pub fn new(nodes_dir: PathBuf) -> Self {
        let keys_dir = nodes_dir.join("keys");
        let _ = std::fs::create_dir_all(&keys_dir);

        Self {
            nodes_dir,
            keys_dir,
        }
    }

    pub fn known_hosts_path(&self) -> PathBuf {
        self.nodes_dir.join("known_hosts")
    }

    pub fn ssh_config_path(&self) -> anyhow::Result<PathBuf> {
        let path = self.nodes_dir.join("config");
        if !path.exists() {
            std::fs::File::create(&path)?;
        }

        Ok(path)
    }

    pub fn get_key_path(&self, node: &Node) -> anyhow::Result<PathBuf> {
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

    pub async fn probe_ssh(&self, node: &Node) -> anyhow::Result<()> {
        let output = self.run_ssh_script(node, "echo ok").await?;

        if output.status.success() {
            Ok(())
        } else {
            self.remove_key(&node.id);
            anyhow::bail!(
                "SSH probe failed: {}",
                String::from_utf8_lossy(&output.stderr)
            )
        }
    }

    pub async fn run_ssh_script(
        &self,
        node: &Node,
        script: &str,
    ) -> anyhow::Result<std::process::Output> {
        let key_path = self.get_key_path(node)?;
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

        let output = child.wait_with_output().await?;
        Ok(output)
    }

    pub async fn run_ssh_script_streaming(
        &self,
        node: &Node,
        script: &str,
        log: &LogHandle,
    ) -> anyhow::Result<String> {
        let key_path = self.get_key_path(node)?;
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
                    log.send(line)
                        .await
                        .map_err(|e| anyhow::anyhow!(e.to_string()))?;
                }
            }
            Ok::<(), anyhow::Error>(())
        };

        let drain_stderr = async {
            if let Some(reader) = stderr {
                let mut lines = reader.lines();
                while let Some(line) = lines.next_line().await? {
                    log.send(line)
                        .await
                        .map_err(|e| anyhow::anyhow!(e.to_string()))?;
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

    pub fn remove_key(&self, node_id: &str) {
        let _ = std::fs::remove_file(self.keys_dir.join(node_id));
    }
}
