use axum::{
    Json,
    extract::{Query, State},
    response::IntoResponse,
};
use serde::Deserialize;
use slasha_db::repos::app_metrics::AppMetricsRepo;

use crate::{HttpResult, extractors::app::ActiveApp, state::Storage};

#[derive(Deserialize)]
pub struct MetricsQuery {
    pub start: Option<chrono::DateTime<chrono::Utc>>,
    pub end: Option<chrono::DateTime<chrono::Utc>>,
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
    )
    .await?;

    Ok(Json(serde_json::json!({ "metrics": metrics })))
}
