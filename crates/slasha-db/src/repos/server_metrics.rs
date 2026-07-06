use diesel::prelude::*;

use crate::{
    connection::DbPool,
    error::DbResult,
    models::{
        schema::server_metrics,
        server_metrics::{NewServerMetrics, ServerMetrics},
    },
};

pub struct ServerMetricsRepo;

impl ServerMetricsRepo {
    pub async fn insert(pool: &DbPool, metrics: NewServerMetrics) -> DbResult<ServerMetrics> {
        let pool = pool.clone();
        tokio::task::spawn_blocking(move || {
            let mut conn = pool.get()?;
            let id = uuid::Uuid::new_v4().to_string();
            diesel::insert_into(server_metrics::table)
                .values((
                    server_metrics::id.eq(&id),
                    server_metrics::cpu_usage.eq(metrics.cpu_usage),
                    server_metrics::memory_used.eq(metrics.memory_used),
                    server_metrics::memory_total.eq(metrics.memory_total),
                    server_metrics::swap_used.eq(metrics.swap_used),
                    server_metrics::swap_total.eq(metrics.swap_total),
                    server_metrics::disk_used.eq(metrics.disk_used),
                    server_metrics::disk_total.eq(metrics.disk_total),
                    server_metrics::network_rx_bps.eq(metrics.network_rx_bps),
                    server_metrics::network_tx_bps.eq(metrics.network_tx_bps),
                    server_metrics::load_average.eq(metrics.load_average),
                ))
                .execute(&mut conn)?;
            Ok(server_metrics::table
                .filter(server_metrics::id.eq(&id))
                .first::<ServerMetrics>(&mut conn)?)
        })
        .await?
    }

    pub async fn find_latest(pool: &DbPool) -> DbResult<Option<ServerMetrics>> {
        let pool = pool.clone();
        tokio::task::spawn_blocking(move || {
            let mut conn = pool.get()?;
            Ok(server_metrics::table
                .order(server_metrics::created_at.desc())
                .first::<ServerMetrics>(&mut conn)
                .optional()?)
        })
        .await?
    }

    pub async fn get_history(pool: &DbPool, hours: i64) -> DbResult<Vec<ServerMetrics>> {
        let pool = pool.clone();
        tokio::task::spawn_blocking(move || {
            let mut conn = pool.get()?;
            let cutoff = chrono::Utc::now().naive_utc() - chrono::Duration::hours(hours);
            Ok(server_metrics::table
                .filter(server_metrics::created_at.ge(cutoff))
                .order(server_metrics::created_at.asc())
                .load::<ServerMetrics>(&mut conn)?)
        })
        .await?
    }

    pub async fn prune_older_than(pool: &DbPool, cutoff: chrono::NaiveDateTime) -> DbResult<usize> {
        let pool = pool.clone();
        tokio::task::spawn_blocking(move || {
            let mut conn = pool.get()?;
            Ok(
                diesel::delete(server_metrics::table.filter(server_metrics::created_at.lt(cutoff)))
                    .execute(&mut conn)?,
            )
        })
        .await?
    }
}
