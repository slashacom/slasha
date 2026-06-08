use diesel::prelude::*;

use crate::{
    connection::DbPool,
    error::DbResult,
    models::{app_metrics::AppMetrics, schema::app_metrics},
};

pub struct AppMetricsRepo;

impl AppMetricsRepo {
    pub async fn insert(pool: &DbPool, metrics: AppMetrics) -> DbResult<AppMetrics> {
        let pool = pool.clone();
        tokio::task::spawn_blocking(move || {
            let mut conn = pool.get()?;
            diesel::insert_into(app_metrics::table)
                .values(&metrics)
                .execute(&mut conn)?;
            Ok(metrics)
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
