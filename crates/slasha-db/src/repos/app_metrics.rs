use duckdb::params;

use crate::{
    connection::DuckdbPool,
    error::{DbError, DbResult},
    models::app_metrics::{AppMetrics, NewAppMetrics},
};

pub struct AppMetricsRepo;

impl AppMetricsRepo {
    pub async fn insert(pool: &DuckdbPool, metrics: NewAppMetrics) -> DbResult<AppMetrics> {
        let pool = pool.clone();
        tokio::task::spawn_blocking(move || {
            let conn = pool.get()?;
            let id = uuid::Uuid::new_v4().to_string();
            conn.execute(
                "INSERT INTO app_metrics (id, app_id, cpu_usage, memory_used, memory_limit, network_rx_bps, network_tx_bps, disk_read_bps, disk_write_bps) VALUES (?, ?, ?, ?, ?, ?, ?, ?, ?)",
                params![
                    id,
                    metrics.app_id,
                    { metrics.cpu_usage },
                    { metrics.memory_used },
                    { metrics.memory_limit },
                    { metrics.network_rx_bps },
                    { metrics.network_tx_bps },
                    { metrics.disk_read_bps },
                    { metrics.disk_write_bps },
                ],
            )?;

            let mut stmt = conn.prepare("SELECT id, app_id, cpu_usage, memory_used, memory_limit, network_rx_bps, network_tx_bps, disk_read_bps, disk_write_bps, created_at FROM app_metrics WHERE id = ?")?;
            let mut iter = stmt.query_map(params![id], |row| {
                Ok(AppMetrics {
                    id: row.get(0)?,
                    app_id: row.get(1)?,
                    cpu_usage: row.get(2)?,
                    memory_used: row.get(3)?,
                    memory_limit: row.get(4)?,
                    network_rx_bps: row.get(5)?,
                    network_tx_bps: row.get(6)?,
                    disk_read_bps: row.get(7)?,
                    disk_write_bps: row.get(8)?,
                    created_at: row.get(9)?,
                })
            })?;

            iter.next().ok_or_else(|| DbError::NotFound("Metric not found".to_string()))?
                .map_err(DbError::Duckdb)
        })
        .await?
    }

    pub async fn get_latest(pool: &DuckdbPool, app_id: &str) -> DbResult<Option<AppMetrics>> {
        let pool = pool.clone();
        let app_id = app_id.to_string();
        tokio::task::spawn_blocking(move || {
            let conn = pool.get()?;
            let mut stmt = conn.prepare("SELECT id, app_id, cpu_usage, memory_used, memory_limit, network_rx_bps, network_tx_bps, disk_read_bps, disk_write_bps, created_at FROM app_metrics WHERE app_id = ? ORDER BY created_at DESC LIMIT 1")?;
            let mut iter = stmt.query_map(params![app_id], |row| {
                Ok(AppMetrics {
                    id: row.get(0)?,
                    app_id: row.get(1)?,
                    cpu_usage: row.get(2)?,
                    memory_used: row.get(3)?,
                    memory_limit: row.get(4)?,
                    network_rx_bps: row.get(5)?,
                    network_tx_bps: row.get(6)?,
                    disk_read_bps: row.get(7)?,
                    disk_write_bps: row.get(8)?,
                    created_at: row.get(9)?,
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
        app_id: &str,
        start: chrono::NaiveDateTime,
        end: chrono::NaiveDateTime,
    ) -> DbResult<Vec<AppMetrics>> {
        let pool = pool.clone();
        let app_id = app_id.to_string();
        tokio::task::spawn_blocking(move || {
            let conn = pool.get()?;
            let mut stmt = conn.prepare("SELECT id, app_id, cpu_usage, memory_used, memory_limit, network_rx_bps, network_tx_bps, disk_read_bps, disk_write_bps, created_at FROM app_metrics WHERE app_id = ? AND created_at >= ? AND created_at <= ? ORDER BY created_at ASC")?;
            let iter = stmt.query_map(params![app_id, start, end], |row| {
                Ok(AppMetrics {
                    id: row.get(0)?,
                    app_id: row.get(1)?,
                    cpu_usage: row.get(2)?,
                    memory_used: row.get(3)?,
                    memory_limit: row.get(4)?,
                    network_rx_bps: row.get(5)?,
                    network_tx_bps: row.get(6)?,
                    disk_read_bps: row.get(7)?,
                    disk_write_bps: row.get(8)?,
                    created_at: row.get(9)?,
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
                "DELETE FROM app_metrics WHERE created_at < ?",
                params![cutoff],
            )?;
            Ok(count)
        })
        .await?
    }

    pub async fn delete_by_app_id(pool: &DuckdbPool, app_id: &str) -> DbResult<usize> {
        let pool = pool.clone();
        let app_id = app_id.to_string();
        tokio::task::spawn_blocking(move || {
            let conn = pool.get()?;
            let count =
                conn.execute("DELETE FROM app_metrics WHERE app_id = ?", params![app_id])?;
            Ok(count)
        })
        .await?
    }
}
