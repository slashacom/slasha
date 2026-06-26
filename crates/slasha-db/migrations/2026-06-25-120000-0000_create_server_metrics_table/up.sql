CREATE TABLE server_metrics (
    id TEXT PRIMARY KEY NOT NULL,
    cpu_usage REAL NOT NULL,
    memory_used INTEGER NOT NULL,
    memory_total INTEGER NOT NULL,
    swap_used INTEGER NOT NULL,
    swap_total INTEGER NOT NULL,
    disk_used INTEGER NOT NULL,
    disk_total INTEGER NOT NULL,
    network_rx_bps REAL NOT NULL,
    network_tx_bps REAL NOT NULL,
    load_average REAL NOT NULL,
    created_at TIMESTAMP NOT NULL DEFAULT CURRENT_TIMESTAMP
);

CREATE INDEX idx_server_metrics_created ON server_metrics(created_at);
