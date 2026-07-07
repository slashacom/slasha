CREATE TABLE app_metrics (
    id VARCHAR PRIMARY KEY,
    app_id VARCHAR NOT NULL,
    cpu_usage DOUBLE NOT NULL,
    memory_used BIGINT NOT NULL,
    memory_limit BIGINT NOT NULL,
    network_rx_bps DOUBLE NOT NULL,
    network_tx_bps DOUBLE NOT NULL,
    disk_read_bps DOUBLE NOT NULL,
    disk_write_bps DOUBLE NOT NULL,
    created_at TIMESTAMP NOT NULL DEFAULT CURRENT_TIMESTAMP
);