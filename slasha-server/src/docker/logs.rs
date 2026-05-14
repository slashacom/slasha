use std::{
    io::Write,
    path::{Path, PathBuf},
    sync::Arc,
};

use bollard::{Docker, query_parameters::LogsOptionsBuilder};
use dashmap::DashMap;
use file_rotate::{
    ContentLimit, FileRotate,
    compression::Compression,
    suffix::{AppendTimestamp, DateFrom, FileLimit},
};
use futures_util::StreamExt;
use tokio::sync::{Mutex, broadcast};

use super::{DeploymentError, DeploymentResult};

const CHANNEL_CAPACITY: usize = 1024;

pub enum LogKey {
    Deployment {
        app_slug: String,
        deployment_id: String,
    },
    Service {
        app_slug: String,
        service_name: String,
    },
}

impl LogKey {
    fn as_map_key(&self) -> String {
        match self {
            LogKey::Deployment {
                app_slug,
                deployment_id,
            } => {
                format!("d:{}:{}", app_slug, deployment_id)
            }
            LogKey::Service {
                app_slug,
                service_name,
            } => {
                format!("s:{}:{}", app_slug, service_name)
            }
        }
    }

    fn as_path(&self, logs_dir: &Path) -> PathBuf {
        match self {
            LogKey::Deployment {
                app_slug,
                deployment_id,
            } => logs_dir
                .join(app_slug)
                .join("deployments")
                .join(deployment_id)
                .join("deployment.log"),
            LogKey::Service {
                app_slug,
                service_name,
            } => logs_dir
                .join(app_slug)
                .join("services")
                .join(service_name)
                .join("service.log"),
        }
    }
}

pub struct LogManager {
    channels: DashMap<String, broadcast::Sender<String>>,
    files: DashMap<String, Arc<Mutex<FileRotate<AppendTimestamp>>>>,
    logs_dir: PathBuf,
}

#[derive(Clone)]
pub struct Log {
    key: String,
    path: PathBuf,
    tx: broadcast::Sender<String>,
    file: Arc<Mutex<FileRotate<AppendTimestamp>>>,
}

impl LogManager {
    pub fn new(logs_dir: PathBuf) -> Self {
        Self {
            channels: DashMap::new(),
            files: DashMap::new(),
            logs_dir,
        }
    }

    pub async fn get_logger(&self, key: &LogKey) -> DeploymentResult<Log> {
        let map_key = key.as_map_key();
        let path = key.as_path(&self.logs_dir);

        self.build_log_handle(map_key, path).await
    }

    async fn build_log_handle(&self, key: String, path: PathBuf) -> DeploymentResult<Log> {
        if let Some(parent) = path.parent() {
            tokio::fs::create_dir_all(parent).await?;
        }

        let tx = self
            .channels
            .entry(key.clone())
            .or_insert_with(|| {
                let (tx, _) = broadcast::channel(CHANNEL_CAPACITY);
                tx
            })
            .clone();

        let file = self
            .files
            .entry(key.clone())
            .or_insert_with(|| {
                Arc::new(Mutex::new(FileRotate::new(
                    &path,
                    AppendTimestamp::with_format(
                        "%Y-%m-%d_%H-%M-%S",
                        FileLimit::MaxFiles(10),
                        DateFrom::Now,
                    ),
                    ContentLimit::Lines(10_000),
                    Compression::None,
                    None,
                )))
            })
            .clone();

        Ok(Log {
            key,
            path,
            tx,
            file,
        })
    }

    pub fn remove(&self, key: &LogKey) {
        let k = key.as_map_key();
        self.channels.remove(&k);
        self.files.remove(&k);
    }
}

impl Log {
    pub async fn send(&self, line: impl Into<String>) -> DeploymentResult<()> {
        let line = line.into();
        let _ = self.tx.send(line.clone()); // no one may be listening
        let mut file = self.file.lock().await;
        writeln!(file, "{line}")?;
        Ok(())
    }

    pub fn subscribe(&self) -> broadcast::Receiver<String> {
        self.tx.subscribe()
    }

    async fn collect_rotated_files(&self) -> DeploymentResult<Vec<PathBuf>> {
        let parent = self.path.parent().ok_or_else(|| {
            DeploymentError::Io(std::io::Error::new(
                std::io::ErrorKind::InvalidInput,
                "log path has no parent directory",
            ))
        })?;

        let base_name = self
            .path
            .file_name()
            .and_then(|n| n.to_str())
            .ok_or_else(|| {
                DeploymentError::Io(std::io::Error::new(
                    std::io::ErrorKind::InvalidInput,
                    "log path has no file name",
                ))
            })?
            .to_string();

        let mut read_dir = tokio::fs::read_dir(parent).await?;
        let mut files: Vec<PathBuf> = Vec::new();

        while let Some(entry) = read_dir.next_entry().await? {
            if entry.file_name().to_string_lossy().starts_with(&base_name) {
                files.push(entry.path());
            }
        }

        files.sort();
        Ok(files)
    }

    pub async fn get_historical(&self) -> DeploymentResult<Vec<String>> {
        let files = self.collect_rotated_files().await?;

        let mut lines = Vec::new();
        for path in files {
            let content = tokio::fs::read_to_string(&path).await?;
            lines.extend(content.lines().map(|s| s.to_string()));
        }

        Ok(lines)
    }

    pub async fn delete_logs(&self) -> DeploymentResult<()> {
        for path in self.collect_rotated_files().await? {
            tokio::fs::remove_file(&path).await?;
        }
        Ok(())
    }

    pub fn key(&self) -> &str {
        &self.key
    }
}

pub async fn stream_container_logs(
    docker_client: Docker,
    log: Log,
    container: String,
    prefix: Option<String>,
) -> DeploymentResult<()> {
    let opts = LogsOptionsBuilder::new()
        .follow(true)
        .stdout(true)
        .stderr(true)
        .build();

    let mut log_stream = docker_client.logs(&container, Some(opts));
    let mut buffer = String::new();

    while let Some(item) = log_stream.next().await {
        match item {
            Ok(output) => {
                let chunk = output.to_string();
                buffer.push_str(&chunk);

                while let Some(pos) = buffer.find('\n') {
                    let line = buffer[..pos].to_string();
                    buffer.drain(..=pos);

                    let formatted = match &prefix {
                        Some(p) => format!("{} {}", p, line),
                        None => line,
                    };
                    log.send(formatted).await?;
                }
            }
            Err(e) => {
                let msg = format!("log stream error for {}: {}", log.key(), e);
                tracing::warn!("{}", msg);
                log.send(msg).await?;
                break;
            }
        }
    }

    if !buffer.is_empty() {
        let formatted = match &prefix {
            Some(p) => format!("{} {}", p, buffer),
            None => buffer,
        };
        log.send(formatted).await?;
    }

    tracing::info!("Runtime log stream ended for {}", log.key());

    Ok(())
}
