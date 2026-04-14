use std::io::Write;
use std::path::PathBuf;
use std::sync::Arc;

use dashmap::DashMap;
use file_rotate::{
    ContentLimit, FileRotate,
    compression::Compression,
    suffix::{AppendTimestamp, DateFrom, FileLimit},
};
use tokio::sync::{Mutex, broadcast};

use crate::error::Result;

const CHANNEL_CAPACITY: usize = 1024;

pub struct DeploymentBroadcaster {
    channels: DashMap<String, broadcast::Sender<String>>,
    files: DashMap<String, Arc<Mutex<FileRotate<AppendTimestamp>>>>,
    logs_dir: PathBuf,
}

impl DeploymentBroadcaster {
    pub fn new(logs_dir: PathBuf) -> Self {
        Self {
            channels: DashMap::new(),
            files: DashMap::new(),
            logs_dir,
        }
    }

    pub async fn get_historical(&self, deployment_id: &str) -> Vec<String> {
        let path = self.logs_dir.join(format!("{deployment_id}.log"));

        if !path.exists() {
            return Vec::new();
        }

        let content = tokio::fs::read_to_string(path).await.unwrap_or_default();

        content.lines().map(|s| s.to_string()).collect()
    }

    pub async fn delete_logs(&self, deployment_id: &str) -> Result<()> {
        self.channels.remove(deployment_id);
        self.files.remove(deployment_id);

        let path = self.logs_dir.join(format!("{deployment_id}.log"));

        if tokio::fs::try_exists(&path).await.unwrap_or(false) {
            tokio::fs::remove_file(path).await?;
        }

        Ok(())
    }

    pub fn subscribe(&self, deployment_id: &str) -> broadcast::Receiver<String> {
        if let Some(sender) = self.channels.get(deployment_id) {
            return sender.subscribe();
        }

        let (tx, rx) = broadcast::channel(CHANNEL_CAPACITY);

        match self.channels.entry(deployment_id.to_string()) {
            dashmap::Entry::Occupied(e) => e.get().subscribe(),
            dashmap::Entry::Vacant(e) => {
                e.insert(tx);
                rx
            }
        }
    }

    pub async fn send(&self, deployment_id: &str, line: String) -> Result<()> {
        if let Some(sender) = self.channels.get(deployment_id) {
            let _ = sender.send(line.clone()); // there may be no one listening 
        }

        let writer = if let Some(entry) = self.files.get(deployment_id) {
            entry.clone()
        } else {
            let path = self.logs_dir.join(format!("{deployment_id}.log"));

            let file_rotate = FileRotate::new(
                path,
                AppendTimestamp::with_format(
                    "%Y-%m-%d_%H-%M-%S",
                    FileLimit::MaxFiles(10),
                    DateFrom::Now,
                ),
                ContentLimit::Lines(10_000),
                Compression::None,
                None,
            );

            let arc = Arc::new(Mutex::new(file_rotate));

            self.files
                .entry(deployment_id.to_string())
                .or_insert(arc.clone());

            arc
        };

        let mut file = writer.lock().await;
        writeln!(file, "{line}")?;

        Ok(())
    }

    pub fn remove(&self, deployment_id: &str) {
        self.channels.remove(deployment_id);
        self.files.remove(deployment_id);
    }
}
