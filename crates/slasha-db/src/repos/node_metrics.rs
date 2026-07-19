use duckdb::params;

use crate::{
    connection::DuckdbPool,
    error::{DbError, DbResult},
    models::node_metrics::{NewNodeMetrics, NodeMetrics},
};

pub struct NodeMetricsRepo;

impl NodeMetricsRepo {
    pub async fn insert(pool: &DuckdbPool, metrics: NewNodeMetrics) -> DbResult<NodeMetrics> {
        let pool = pool.clone();
        tokio::task::spawn_blocking(move || {
            let conn = pool.get()?;
            let id = uuid::Uuid::new_v4().to_string();
            conn.execute(
                "INSERT INTO node_metrics ( \
                    id, \
                    node_id, \
                    cpu_usage, \
                    memory_used, \
                    memory_total, \
                    swap_used, \
                    swap_total, \
                    disk_used, \
                    disk_total, \
                    network_rx_bps, \
                    network_tx_bps, \
                    load_average \
                 ) VALUES (?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?)",
                params![
                    id,
                    { metrics.node_id },
                    { metrics.cpu_usage },
                    { metrics.memory_used },
                    { metrics.memory_total },
                    { metrics.swap_used },
                    { metrics.swap_total },
                    { metrics.disk_used },
                    { metrics.disk_total },
                    { metrics.network_rx_bps },
                    { metrics.network_tx_bps },
                    { metrics.load_average }
                ],
            )?;

            let mut stmt = conn.prepare(
                "SELECT \
                    id, \
                    node_id, \
                    cpu_usage, \
                    memory_used, \
                    memory_total, \
                    swap_used, \
                    swap_total, \
                    disk_used, \
                    disk_total, \
                    network_rx_bps, \
                    network_tx_bps, \
                    load_average, \
                    created_at \
                 FROM node_metrics \
                 WHERE id = ?",
            )?;
            let mut iter = stmt.query_map(params![id], |row| {
                Ok(NodeMetrics {
                    id: row.get(0)?,
                    node_id: row.get(1)?,
                    cpu_usage: row.get(2)?,
                    memory_used: row.get(3)?,
                    memory_total: row.get(4)?,
                    swap_used: row.get(5)?,
                    swap_total: row.get(6)?,
                    disk_used: row.get(7)?,
                    disk_total: row.get(8)?,
                    network_rx_bps: row.get(9)?,
                    network_tx_bps: row.get(10)?,
                    load_average: row.get(11)?,
                    created_at: row.get(12)?,
                })
            })?;

            iter.next()
                .ok_or_else(|| DbError::NotFound("Metric not found".to_string()))?
                .map_err(DbError::Duckdb)
        })
        .await?
    }

    pub async fn get_latest(pool: &DuckdbPool, node_id: &str) -> DbResult<Option<NodeMetrics>> {
        let pool = pool.clone();
        let node_id = node_id.to_string();
        tokio::task::spawn_blocking(move || {
            let conn = pool.get()?;
            let mut stmt = conn.prepare(
                "SELECT \
                    id, \
                    node_id, \
                    cpu_usage, \
                    memory_used, \
                    memory_total, \
                    swap_used, \
                    swap_total, \
                    disk_used, \
                    disk_total, \
                    network_rx_bps, \
                    network_tx_bps, \
                    load_average, \
                    created_at \
                 FROM node_metrics \
                 WHERE node_id = ? \
                 ORDER BY created_at DESC \
                 LIMIT 1",
            )?;
            let mut iter = stmt.query_map(params![node_id], |row| {
                Ok(NodeMetrics {
                    id: row.get(0)?,
                    node_id: row.get(1)?,
                    cpu_usage: row.get(2)?,
                    memory_used: row.get(3)?,
                    memory_total: row.get(4)?,
                    swap_used: row.get(5)?,
                    swap_total: row.get(6)?,
                    disk_used: row.get(7)?,
                    disk_total: row.get(8)?,
                    network_rx_bps: row.get(9)?,
                    network_tx_bps: row.get(10)?,
                    load_average: row.get(11)?,
                    created_at: row.get(12)?,
                })
            })?;

            if let Some(res) = iter.next() {
                Ok(Some(res?))
            } else {
                Ok(None)
            }
        })
        .await?
    }

    pub async fn get_history(
        pool: &DuckdbPool,
        node_id: &str,
        start: chrono::NaiveDateTime,
        end: chrono::NaiveDateTime,
        bucket_seconds: i64,
    ) -> DbResult<Vec<NodeMetrics>> {
        let pool = pool.clone();
        let bucket = bucket_seconds.max(1);
        let node_id = node_id.to_string();
        tokio::task::spawn_blocking(move || {
            let conn = pool.get()?;
            let sql = format!(
                "SELECT \
                    CAST(time_bucket(to_seconds({bucket}), created_at) AS VARCHAR) AS id, \
                    node_id, \
                    avg(cpu_usage) AS cpu_usage, \
                    CAST(avg(memory_used) AS BIGINT) AS memory_used, \
                    CAST(max(memory_total) AS BIGINT) AS memory_total, \
                    CAST(avg(swap_used) AS BIGINT) AS swap_used, \
                    CAST(max(swap_total) AS BIGINT) AS swap_total, \
                    CAST(avg(disk_used) AS BIGINT) AS disk_used, \
                    CAST(max(disk_total) AS BIGINT) AS disk_total, \
                    avg(network_rx_bps) AS network_rx_bps, \
                    avg(network_tx_bps) AS network_tx_bps, \
                    avg(load_average) AS load_average, \
                    time_bucket(to_seconds({bucket}), created_at) AS created_at \
                 FROM node_metrics \
                 WHERE node_id = ? AND created_at >= ? AND created_at <= ? \
                 GROUP BY ALL \
                 ORDER BY created_at ASC"
            );

            let mut stmt = conn.prepare(&sql)?;

            let iter = stmt.query_map(params![node_id, start, end], |row| {
                Ok(NodeMetrics {
                    id: row.get(0)?,
                    node_id: row.get(1)?,
                    cpu_usage: row.get(2)?,
                    memory_used: row.get(3)?,
                    memory_total: row.get(4)?,
                    swap_used: row.get(5)?,
                    swap_total: row.get(6)?,
                    disk_used: row.get(7)?,
                    disk_total: row.get(8)?,
                    network_rx_bps: row.get(9)?,
                    network_tx_bps: row.get(10)?,
                    load_average: row.get(11)?,
                    created_at: row.get(12)?,
                })
            })?;

            let mut results = Vec::new();
            for item in iter {
                results.push(item?);
            }
            Ok(results)
        })
        .await?
    }

    pub async fn prune_older_than(
        pool: &DuckdbPool,
        cutoff: chrono::NaiveDateTime,
    ) -> DbResult<usize> {
        let pool = pool.clone();
        tokio::task::spawn_blocking(move || {
            let conn = pool.get()?;
            let count = conn.execute(
                "DELETE FROM node_metrics WHERE created_at < ?",
                params![cutoff],
            )?;
            Ok(count)
        })
        .await?
    }
}
