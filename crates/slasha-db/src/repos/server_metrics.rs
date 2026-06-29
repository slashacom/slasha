use diesel::prelude::*;

use crate::{
    connection::DbPool,
    error::DbResult,
    models::{schema::server_metrics, server_metrics::ServerMetrics},
};

pub struct ServerMetricsRepo;

impl ServerMetricsRepo {
    pub async fn insert(pool: &DbPool, metrics: ServerMetrics) -> DbResult<ServerMetrics> {
        let pool = pool.clone();
        tokio::task::spawn_blocking(move || {
            let mut conn = pool.get()?;
            diesel::insert_into(server_metrics::table)
                .values(&metrics)
                .execute(&mut conn)?;
            Ok(metrics)
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
