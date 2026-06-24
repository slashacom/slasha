use axum::{
    Json,
    extract::{Query, State},
    response::IntoResponse,
};
use serde::Deserialize;
use slasha_db::repos::app_metrics::AppMetricsRepo;

use crate::{error::HttpResult, extractors::app::ActiveApp, state::Storage};

#[derive(Deserialize)]
pub struct MetricsQuery {
    pub hours: Option<i64>,
}

pub async fn get_metrics(
    State(storage): State<Storage>,
    ActiveApp { app, .. }: ActiveApp,
    Query(query): Query<MetricsQuery>,
) -> HttpResult<impl IntoResponse> {
    let hours = query.hours.unwrap_or(168); // 7 days
    let metrics = AppMetricsRepo::get_history(&storage.db_pool, &app.id, hours).await?;

    Ok(Json(serde_json::json!({ "metrics": metrics })))
}
