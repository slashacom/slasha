-- remove all indexes from duckdb metrics and logs tables to avoid ART index
-- deletion crashes during background pruning. The metrics tables had primary-key
-- indexes and logs had a primary-key index plus additional
-- indexes, all of which may be affected by this issue.

-- https://github.com/duckdb/duckdb/issues/23645

CREATE TABLE app_metrics_tmp (
    id             VARCHAR,
    app_id         VARCHAR NOT NULL,
    cpu_usage      DOUBLE NOT NULL,
    memory_used    BIGINT NOT NULL,
    memory_limit   BIGINT NOT NULL,
    network_rx_bps DOUBLE NOT NULL,
    network_tx_bps DOUBLE NOT NULL,
    disk_read_bps  DOUBLE NOT NULL,
    disk_write_bps DOUBLE NOT NULL,
    created_at     TIMESTAMP NOT NULL DEFAULT CURRENT_TIMESTAMP
);

INSERT INTO app_metrics_tmp (
    id, app_id, cpu_usage, memory_used, memory_limit,
    network_rx_bps, network_tx_bps, disk_read_bps, disk_write_bps, created_at
)
SELECT
    id, app_id, cpu_usage, memory_used, memory_limit,
    network_rx_bps, network_tx_bps, disk_read_bps, disk_write_bps, created_at
FROM app_metrics;

DROP TABLE app_metrics;
ALTER TABLE app_metrics_tmp RENAME TO app_metrics;

CREATE TABLE node_metrics_tmp (
    id             VARCHAR,
    node_id        VARCHAR,
    cpu_usage      DOUBLE NOT NULL,
    memory_used    BIGINT NOT NULL,
    memory_total   BIGINT NOT NULL,
    swap_used      BIGINT NOT NULL,
    swap_total     BIGINT NOT NULL,
    disk_used      BIGINT NOT NULL,
    disk_total     BIGINT NOT NULL,
    network_rx_bps DOUBLE NOT NULL,
    network_tx_bps DOUBLE NOT NULL,
    load_average   DOUBLE NOT NULL,
    created_at     TIMESTAMP NOT NULL DEFAULT CURRENT_TIMESTAMP
);

INSERT INTO node_metrics_tmp (
    id, node_id, cpu_usage, memory_used, memory_total,
    swap_used, swap_total, disk_used, disk_total,
    network_rx_bps, network_tx_bps, load_average, created_at
)
SELECT
    id, node_id, cpu_usage, memory_used, memory_total,
    swap_used, swap_total, disk_used, disk_total,
    network_rx_bps, network_tx_bps, load_average, created_at
FROM node_metrics;

DROP TABLE node_metrics;
ALTER TABLE node_metrics_tmp RENAME TO node_metrics;

CREATE TABLE logs_tmp (
    id            VARCHAR,
    timestamp     TIMESTAMP NOT NULL DEFAULT CURRENT_TIMESTAMP,
    resource_kind VARCHAR NOT NULL,
    resource_id   VARCHAR NOT NULL,
    app_id        VARCHAR,
    prefix        VARCHAR,
    stream        VARCHAR NOT NULL,
    message       VARCHAR NOT NULL
);

INSERT INTO logs_tmp (
    id, timestamp, resource_kind, resource_id, app_id, prefix, stream, message
)
SELECT
    id, timestamp, resource_kind, resource_id, app_id, prefix, stream, message
FROM logs;

DROP TABLE logs;
ALTER TABLE logs_tmp RENAME TO logs;