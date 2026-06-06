CREATE TABLE app_metrics (
    id TEXT PRIMARY KEY NOT NULL,
    app_id TEXT NOT NULL REFERENCES apps(id) ON DELETE CASCADE,
    cpu_usage REAL NOT NULL,
    memory_used INTEGER NOT NULL,
    memory_limit INTEGER NOT NULL,
    network_rx_bps REAL NOT NULL,
    network_tx_bps REAL NOT NULL,
    disk_read_bps REAL NOT NULL,
    disk_write_bps REAL NOT NULL,
    created_at TIMESTAMP NOT NULL DEFAULT CURRENT_TIMESTAMP
);

CREATE INDEX idx_app_metrics_app_id_created ON app_metrics(app_id, created_at);
