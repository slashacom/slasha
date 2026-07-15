use axum::{
    Json,
    extract::{Query, State},
    response::IntoResponse,
};
use serde::Deserialize;
use slasha_db::repos::app_metrics::AppMetricsRepo;

use crate::{HttpResult, extractors::app::ActiveApp, state::Storage};

const RAW_INTERVAL_SECONDS: i64 = 10;
const TARGET_POINTS: i64 = 240;

#[derive(Deserialize)]
pub struct MetricsQuery {
    pub start: Option<chrono::DateTime<chrono::Utc>>,
    pub end: Option<chrono::DateTime<chrono::Utc>>,
}

fn bucket_seconds(start: chrono::DateTime<chrono::Utc>, end: chrono::DateTime<chrono::Utc>) -> i64 {
    let span = (end - start).num_seconds().max(RAW_INTERVAL_SECONDS);
    let intervals = (span / TARGET_POINTS / RAW_INTERVAL_SECONDS).max(1);
    intervals * RAW_INTERVAL_SECONDS
}

pub async fn get_metrics(
    State(storage): State<Storage>,
    ActiveApp { app, .. }: ActiveApp,
    Query(query): Query<MetricsQuery>,
) -> HttpResult<impl IntoResponse> {
    let end = query.end.unwrap_or_else(chrono::Utc::now);
    let start = query
        .start
        .unwrap_or_else(|| end - chrono::Duration::hours(24));

    let metrics = AppMetricsRepo::get_history(
        &storage.duckdb_pool,
        &app.id,
        start.naive_utc(),
        end.naive_utc(),
        bucket_seconds(start, end),
    )
    .await?;

    Ok(Json(serde_json::json!({ "metrics": metrics })))
}

pub async fn get_latest_metric(
    State(storage): State<Storage>,
    ActiveApp { app, .. }: ActiveApp,
) -> HttpResult<impl IntoResponse> {
    let metric = AppMetricsRepo::get_latest(&storage.duckdb_pool, &app.id).await?;

    Ok(Json(serde_json::json!({ "metric": metric })))
}
