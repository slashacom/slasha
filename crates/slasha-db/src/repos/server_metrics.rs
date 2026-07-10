use duckdb::params;

use crate::{
    connection::DuckdbPool,
    error::{DbError, DbResult},
    models::server_metrics::{NewServerMetrics, ServerMetrics},
};

pub struct ServerMetricsRepo;

impl ServerMetricsRepo {
    pub async fn insert(pool: &DuckdbPool, metrics: NewServerMetrics) -> DbResult<ServerMetrics> {
        let pool = pool.clone();
        tokio::task::spawn_blocking(move || {
            let conn = pool.get()?;
            let id = uuid::Uuid::new_v4().to_string();
            conn.execute(
                "INSERT INTO server_metrics (id, cpu_usage, memory_used, memory_total, swap_used, swap_total, disk_used, disk_total, network_rx_bps, network_tx_bps, load_average) VALUES (?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?)",
                params![
                    id,
                    { metrics.cpu_usage },
                    { metrics.memory_used },
                    { metrics.memory_total },
                    { metrics.swap_used },
                    { metrics.swap_total },
                    { metrics.disk_used },
                    { metrics.disk_total },
                    { metrics.network_rx_bps },
                    { metrics.network_tx_bps },
                    { metrics.load_average },
                ],
            )?;

            let mut stmt = conn.prepare("SELECT id, cpu_usage, memory_used, memory_total, swap_used, swap_total, disk_used, disk_total, network_rx_bps, network_tx_bps, load_average, created_at FROM server_metrics WHERE id = ?")?;
            let mut iter = stmt.query_map(params![id], |row| {
                Ok(ServerMetrics {
                    id: row.get(0)?,
                    cpu_usage: row.get(1)?,
                    memory_used: row.get(2)?,
                    memory_total: row.get(3)?,
                    swap_used: row.get(4)?,
                    swap_total: row.get(5)?,
                    disk_used: row.get(6)?,
                    disk_total: row.get(7)?,
                    network_rx_bps: row.get(8)?,
                    network_tx_bps: row.get(9)?,
                    load_average: row.get(10)?,
                    created_at: row.get(11)?,
                })
            })?;

            iter.next().ok_or_else(|| DbError::NotFound("Metric not found".to_string()))?
                .map_err(DbError::Duckdb)
        })
        .await?
    }

    pub async fn get_latest(pool: &DuckdbPool) -> DbResult<Option<ServerMetrics>> {
        let pool = pool.clone();
        tokio::task::spawn_blocking(move || {
            let conn = pool.get()?;
            let mut stmt = conn.prepare("SELECT id, cpu_usage, memory_used, memory_total, swap_used, swap_total, disk_used, disk_total, network_rx_bps, network_tx_bps, load_average, created_at FROM server_metrics ORDER BY created_at DESC LIMIT 1")?;
            let mut iter = stmt.query_map([], |row| {
                Ok(ServerMetrics {
                    id: row.get(0)?,
                    cpu_usage: row.get(1)?,
                    memory_used: row.get(2)?,
                    memory_total: row.get(3)?,
                    swap_used: row.get(4)?,
                    swap_total: row.get(5)?,
                    disk_used: row.get(6)?,
                    disk_total: row.get(7)?,
                    network_rx_bps: row.get(8)?,
                    network_tx_bps: row.get(9)?,
                    load_average: row.get(10)?,
                    created_at: row.get(11)?,
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
        start: chrono::NaiveDateTime,
        end: chrono::NaiveDateTime,
        bucket_seconds: i64,
    ) -> DbResult<Vec<ServerMetrics>> {
        let pool = pool.clone();
        let bucket = bucket_seconds.max(1);
        tokio::task::spawn_blocking(move || {
            let conn = pool.get()?;
            let sql = format!(
                "SELECT \
                    CAST(time_bucket(to_seconds({bucket}), created_at) AS VARCHAR) AS id, \
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
                 FROM server_metrics \
                 WHERE created_at >= ? AND created_at <= ? \
                 GROUP BY ALL \
                 ORDER BY created_at ASC"
            );
            let mut stmt = conn.prepare(&sql)?;
            let iter = stmt.query_map(params![start, end], |row| {
                Ok(ServerMetrics {
                    id: row.get(0)?,
                    cpu_usage: row.get(1)?,
                    memory_used: row.get(2)?,
                    memory_total: row.get(3)?,
                    swap_used: row.get(4)?,
                    swap_total: row.get(5)?,
                    disk_used: row.get(6)?,
                    disk_total: row.get(7)?,
                    network_rx_bps: row.get(8)?,
                    network_tx_bps: row.get(9)?,
                    load_average: row.get(10)?,
                    created_at: row.get(11)?,
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
                "DELETE FROM server_metrics WHERE created_at < ?",
                params![cutoff],
            )?;
            Ok(count)
        })
        .await?
    }
}
