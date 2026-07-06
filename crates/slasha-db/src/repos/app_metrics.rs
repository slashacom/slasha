use diesel::prelude::*;

use crate::{
    connection::DbPool,
    error::DbResult,
    models::{
        app_metrics::{AppMetrics, NewAppMetrics},
        schema::app_metrics,
    },
};

pub struct AppMetricsRepo;

impl AppMetricsRepo {
    pub async fn insert(pool: &DbPool, metrics: NewAppMetrics) -> DbResult<AppMetrics> {
        let pool = pool.clone();
        tokio::task::spawn_blocking(move || {
            let mut conn = pool.get()?;
            let id = uuid::Uuid::new_v4().to_string();
            diesel::insert_into(app_metrics::table)
                .values((
                    app_metrics::id.eq(&id),
                    app_metrics::app_id.eq(&metrics.app_id),
                    app_metrics::cpu_usage.eq(metrics.cpu_usage),
                    app_metrics::memory_used.eq(metrics.memory_used),
                    app_metrics::memory_limit.eq(metrics.memory_limit),
                    app_metrics::network_rx_bps.eq(metrics.network_rx_bps),
                    app_metrics::network_tx_bps.eq(metrics.network_tx_bps),
                    app_metrics::disk_read_bps.eq(metrics.disk_read_bps),
                    app_metrics::disk_write_bps.eq(metrics.disk_write_bps),
                ))
                .execute(&mut conn)?;
            Ok(app_metrics::table
                .filter(app_metrics::id.eq(&id))
                .first::<AppMetrics>(&mut conn)?)
        })
        .await?
    }

    pub async fn find_latest(pool: &DbPool, app_id: &str) -> DbResult<Option<AppMetrics>> {
        let pool = pool.clone();
        let app_id = app_id.to_string();
        tokio::task::spawn_blocking(move || {
            let mut conn = pool.get()?;
            Ok(app_metrics::table
                .filter(app_metrics::app_id.eq(&app_id))
                .order(app_metrics::created_at.desc())
                .first::<AppMetrics>(&mut conn)
                .optional()?)
        })
        .await?
    }

    pub async fn get_history(pool: &DbPool, app_id: &str, hours: i64) -> DbResult<Vec<AppMetrics>> {
        let pool = pool.clone();
        let app_id = app_id.to_string();
        tokio::task::spawn_blocking(move || {
            let mut conn = pool.get()?;
            let cutoff = chrono::Utc::now().naive_utc() - chrono::Duration::hours(hours);
            Ok(app_metrics::table
                .filter(app_metrics::app_id.eq(&app_id))
                .filter(app_metrics::created_at.ge(cutoff))
                .order(app_metrics::created_at.asc())
                .load::<AppMetrics>(&mut conn)?)
        })
        .await?
    }

    pub async fn prune_older_than(pool: &DbPool, cutoff: chrono::NaiveDateTime) -> DbResult<usize> {
        let pool = pool.clone();
        tokio::task::spawn_blocking(move || {
            let mut conn = pool.get()?;
            Ok(
                diesel::delete(app_metrics::table.filter(app_metrics::created_at.lt(cutoff)))
                    .execute(&mut conn)?,
            )
        })
        .await?
    }
}
