use std::path::PathBuf;
use std::sync::Arc;

use dashmap::DashMap;
use tokio::fs;
use tokio::fs::OpenOptions;
use tokio::io::AsyncWriteExt;
use tokio::sync::{Mutex, broadcast};

use crate::error::Result;

const CHANNEL_CAPACITY: usize = 1024;

pub struct DeploymentBroadcaster {
    channels: DashMap<String, broadcast::Sender<String>>,
    files: DashMap<String, Arc<Mutex<fs::File>>>,
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
        let path = self.logs_dir.join(format!("{}.log", deployment_id));
        if !path.exists() {
            return Vec::new();
        }

        let content = fs::read_to_string(path).await.unwrap_or_default();
        content.lines().map(|line| line.to_string()).collect()
    }

    pub async fn delete_logs(&self, deployment_id: &str) -> Result<()> {
        self.channels.remove(deployment_id);
        self.files.remove(deployment_id);

        let path = self.logs_dir.join(format!("{}.log", deployment_id));

        if fs::try_exists(&path).await.unwrap_or(false) {
            fs::remove_file(&path).await?;
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
            let _ = sender.send(line.clone()); // ignore if no one is listening
        }

        let file = if let Some(file_entry) = self.files.get(deployment_id) {
            file_entry.clone()
        } else {
            let f = OpenOptions::new()
                .create(true)
                .append(true)
                .open(self.logs_dir.join(format!("{}.log", deployment_id)))
                .await?;

            let file_arc = Arc::new(Mutex::new(f));
            self.files
                .entry(deployment_id.to_string())
                .or_insert(file_arc.clone())
                .clone()
        };

        let mut file = file.lock().await;
        file.write_all(line.as_bytes()).await?;
        file.write_all(b"\n").await?;

        Ok(())
    }

    pub fn remove(&self, deployment_id: &str) {
        self.channels.remove(deployment_id);
        self.files.remove(deployment_id);
    }
}
