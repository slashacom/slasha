use axum::{
    Json, Router,
    extract::{Query, State},
    response::IntoResponse,
    routing::get,
};
use serde::Deserialize;
use slasha_db::repos::server_metrics::ServerMetricsRepo;

use crate::{AppState, HttpResult, extractors::auth::AuthUser, state::Storage};

#[derive(Deserialize)]
pub struct MetricsQuery {
    pub start: Option<chrono::DateTime<chrono::Utc>>,
    pub end: Option<chrono::DateTime<chrono::Utc>>,
}

/// Collection cadence of the metrics sampler, in seconds. Buckets never go
/// finer than this since there is no more data to resolve.
const RAW_INTERVAL_SECONDS: i64 = 15;
/// Roughly how many points to return for any span, so charts stay smooth.
const TARGET_POINTS: i64 = 240;

pub fn router() -> Router<AppState> {
    Router::new()
        .route("/metrics", get(get_metrics))
        .route("/metrics/latest", get(get_latest_metrics))
}

/// Picks a bucket width that keeps the point count near `TARGET_POINTS`,
/// aligned to the raw collection interval.
fn bucket_seconds(start: chrono::DateTime<chrono::Utc>, end: chrono::DateTime<chrono::Utc>) -> i64 {
    let span = (end - start).num_seconds().max(RAW_INTERVAL_SECONDS);
    let intervals = (span / TARGET_POINTS / RAW_INTERVAL_SECONDS).max(1);
    intervals * RAW_INTERVAL_SECONDS
}

async fn get_metrics(
    State(storage): State<Storage>,
    AuthUser(_user): AuthUser,
    Query(query): Query<MetricsQuery>,
) -> HttpResult<impl IntoResponse> {
    let end = query.end.unwrap_or_else(chrono::Utc::now);
    let start = query
        .start
        .unwrap_or_else(|| end - chrono::Duration::hours(24));

    let metrics = ServerMetricsRepo::get_history(
        &storage.duckdb_pool,
        start.naive_utc(),
        end.naive_utc(),
        bucket_seconds(start, end),
    )
    .await?;

    Ok(Json(serde_json::json!({ "metrics": metrics })))
}

async fn get_latest_metrics(
    State(storage): State<Storage>,
    AuthUser(_user): AuthUser,
) -> HttpResult<impl IntoResponse> {
    let metric = ServerMetricsRepo::get_latest(&storage.duckdb_pool).await?;

    Ok(Json(serde_json::json!({ "metric": metric })))
}
