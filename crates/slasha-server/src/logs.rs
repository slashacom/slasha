use std::{sync::Arc, time::Duration};

use dashmap::DashMap;
use slasha_db::{
    DuckdbPool,
    models::logs::{LogPrefix, LogRecord, LogStream, ResourceKind},
    repos::logs::LogsRepo,
};
use tokio::sync::{broadcast, mpsc};

const CHANNEL_CAPACITY: usize = 1024;
const BATCH_INTERVAL_MS: u64 = 100;
const MAX_BATCH_SIZE: usize = 500;

#[derive(Clone)]
pub struct LogBus {
    channels: Arc<DashMap<String, broadcast::Sender<LogRecord>>>,
    batch_tx: mpsc::UnboundedSender<LogRecord>,
}

/// Contextual log builder that automatically fills resource metadata and timestamps.
#[derive(Clone)]
pub struct LogWriter {
    bus: LogBus,
    pub resource_kind: ResourceKind,
    pub resource_id: String,
    pub app_id: Option<String>,
    pub prefix: Option<LogPrefix>,
}

impl LogBus {
    /// Creates a new [`LogBus`] instance and spawns the background DuckDB flusher task.
    ///
    /// # Arguments
    ///
    /// * `pool` - DuckDB connection pool ([`DuckdbPool`]).
    ///
    /// # Returns
    ///
    /// A new [`LogBus`] instance.
    pub fn new(pool: DuckdbPool) -> Self {
        let (batch_tx, batch_rx) = mpsc::unbounded_channel();
        let bus = Self {
            channels: Arc::new(DashMap::new()),
            batch_tx,
        };

        Self::spawn_flusher(pool, batch_rx);
        bus
    }

    /// Creates a contextual [`LogWriter`] pre-populated with resource metadata.
    ///
    /// # Arguments
    ///
    /// * `resource_kind` - The kind of resource ([`ResourceKind`]).
    /// * `resource_id` - Unique identifier of the resource.
    ///
    /// # Returns
    ///
    /// A contextual [`LogWriter`].
    pub fn writer(&self, resource_kind: ResourceKind, resource_id: impl Into<String>) -> LogWriter {
        LogWriter {
            bus: self.clone(),
            resource_kind,
            resource_id: resource_id.into(),
            app_id: None,
            prefix: Some(LogPrefix::System),
        }
    }

    /// Publishes a [`LogRecord`] to active live subscribers and queues it for DuckDB batch insertion.
    ///
    /// # Arguments
    ///
    /// * `record` - The log record to publish.
    pub fn publish(&self, record: LogRecord) {
        if let Some(entry) = self.channels.get(&record.resource_id) {
            if entry.value().receiver_count() > 0 {
                let _ = entry.value().send(record.clone());
            } else {
                drop(entry);
                self.channels
                    .remove_if(&record.resource_id, |_, tx| tx.receiver_count() == 0);
            }
        }

        let _ = self.batch_tx.send(record);
    }

    /// Subscribes to live log records for a specific resource ID.
    ///
    /// # Arguments
    ///
    /// * `resource_id` - Unique identifier of the resource.
    ///
    /// # Returns
    ///
    /// A [`broadcast::Receiver<LogRecord>`] stream handle.
    pub fn subscribe(&self, resource_id: &str) -> broadcast::Receiver<LogRecord> {
        self.channels
            .entry(resource_id.to_string())
            .or_insert_with(|| {
                let (tx, _) = broadcast::channel(CHANNEL_CAPACITY);
                tx
            })
            .subscribe()
    }

    /// Drops the in-memory broadcast channel for a resource.
    ///
    /// # Arguments
    ///
    /// * `resource_id` - Resource ID whose broadcast channel should be closed.
    pub fn remove(&self, resource_id: &str) {
        self.channels.remove(resource_id);
    }

    /// Spawns a background task that flushes queued log records to DuckDB every 100ms.
    fn spawn_flusher(pool: DuckdbPool, mut rx: mpsc::UnboundedReceiver<LogRecord>) {
        async fn flush(pool: &DuckdbPool, buffer: &mut Vec<LogRecord>) {
            if buffer.is_empty() {
                return;
            }

            let batch = std::mem::take(buffer);

            if let Err(err) = LogsRepo::insert_batch(pool, batch).await {
                tracing::error!(error = ?err, "failed to insert log batch to duckdb");
            }
        }

        tokio::spawn(async move {
            let mut interval = tokio::time::interval(Duration::from_millis(BATCH_INTERVAL_MS));
            let mut buffer = Vec::with_capacity(MAX_BATCH_SIZE);

            loop {
                tokio::select! {
                    _ = interval.tick() => {
                        flush(&pool, &mut buffer).await;
                    }

                    item = rx.recv() => {
                        match item {
                            Some(record) => {
                                buffer.push(record);

                                if buffer.len() >= MAX_BATCH_SIZE {
                                    flush(&pool, &mut buffer).await;
                                }
                            }
                            // channel closed, drain buffer and exit
                            None => {
                                flush(&pool, &mut buffer).await;
                                break;
                            }
                        }
                    }
                }
            }
        });
    }
}

impl LogWriter {
    /// Binds an app ID to the log writer for cascading deletes and app filtering.
    pub fn app_id(mut self, app_id: impl Into<String>) -> Self {
        self.app_id = Some(app_id.into());
        self
    }

    /// Add a prefix to the emitted log records.
    pub fn prefix(mut self, prefix: LogPrefix) -> Self {
        self.prefix = Some(prefix);
        self
    }

    /// Emits a stdout log record.
    pub fn stdout(&self, message: impl Into<String>) {
        self.send(LogStream::Stdout, message);
    }

    /// Emits a stderr log record.
    pub fn stderr(&self, message: impl Into<String>) {
        self.send(LogStream::Stderr, message);
    }

    /// Emits a log record with explicit stream classification.
    fn send(&self, stream: LogStream, message: impl Into<String>) {
        let record = LogRecord {
            id: uuid::Uuid::new_v4().to_string(),
            timestamp: chrono::Utc::now().naive_utc(),
            resource_kind: self.resource_kind,
            resource_id: self.resource_id.clone(),
            app_id: self.app_id.clone(),
            prefix: self.prefix.clone(),
            stream,
            message: message.into(),
        };

        self.bus.publish(record);
    }
}
