CREATE TABLE server_metrics (
    id VARCHAR PRIMARY KEY,
    cpu_usage DOUBLE NOT NULL,
    memory_used BIGINT NOT NULL,
    memory_total BIGINT NOT NULL,
    swap_used BIGINT NOT NULL,
    swap_total BIGINT NOT NULL,
    disk_used BIGINT NOT NULL,
    disk_total BIGINT NOT NULL,
    network_rx_bps DOUBLE NOT NULL,
    network_tx_bps DOUBLE NOT NULL,
    load_average DOUBLE NOT NULL,
    created_at TIMESTAMP NOT NULL DEFAULT CURRENT_TIMESTAMP
);