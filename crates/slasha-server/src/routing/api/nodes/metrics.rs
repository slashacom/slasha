use axum::{
    Json, Router,
    extract::{Path, Query, State},
    response::IntoResponse,
    routing::get,
};
use serde::Deserialize;
use slasha_db::repos::node_metrics::NodeMetricsRepo;

use crate::{
    HttpResult,
    extractors::auth::AuthUser,
    state::{AppState, Storage},
};

#[derive(Deserialize)]
pub struct MetricsQuery {
    pub start: Option<chrono::DateTime<chrono::Utc>>,
    pub end: Option<chrono::DateTime<chrono::Utc>>,
}

pub fn router() -> Router<AppState> {
    Router::new()
        .route("/{id}/metrics", get(get_node_metrics))
        .route("/{id}/metrics/latest", get(get_latest_metrics))
}

const RAW_INTERVAL_SECONDS: i64 = 15;
const TARGET_POINTS: i64 = 240;

fn bucket_seconds(start: chrono::DateTime<chrono::Utc>, end: chrono::DateTime<chrono::Utc>) -> i64 {
    let span = (end - start).num_seconds().max(RAW_INTERVAL_SECONDS);
    let intervals = (span / TARGET_POINTS / RAW_INTERVAL_SECONDS).max(1);
    intervals * RAW_INTERVAL_SECONDS
}

pub async fn get_node_metrics(
    State(storage): State<Storage>,
    AuthUser(_user): AuthUser,
    Path(id): Path<String>,
    Query(query): Query<MetricsQuery>,
) -> HttpResult<impl IntoResponse> {
    let end = query.end.unwrap_or_else(chrono::Utc::now);
    let start = query
        .start
        .unwrap_or_else(|| end - chrono::Duration::hours(24));

    let metrics = NodeMetricsRepo::get_history(
        &storage.duckdb_pool,
        &id,
        start.naive_utc(),
        end.naive_utc(),
        bucket_seconds(start, end),
    )
    .await?;

    Ok(Json(serde_json::json!({ "metrics": metrics })))
}

pub async fn get_latest_metrics(
    State(storage): State<Storage>,
    AuthUser(_user): AuthUser,
    Path(id): Path<String>,
) -> HttpResult<impl IntoResponse> {
    let metric = NodeMetricsRepo::get_latest(&storage.duckdb_pool, &id).await?;

    Ok(Json(serde_json::json!({ "metric": metric })))
}
