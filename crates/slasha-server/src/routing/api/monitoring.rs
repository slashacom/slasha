use axum::{
    Json, Router,
    extract::{Query, State},
    response::IntoResponse,
    routing::get,
};
use serde::Deserialize;
use slasha_db::repos::server_metrics::ServerMetricsRepo;

use crate::{AppState, error::HttpResult, extractors::auth::AuthUser, state::Storage};

#[derive(Deserialize)]
pub struct MetricsQuery {
    pub hours: Option<i64>,
}

pub fn router() -> Router<AppState> {
    Router::new().route("/metrics", get(get_metrics))
}

async fn get_metrics(
    State(storage): State<Storage>,
    AuthUser(_user): AuthUser,
    Query(query): Query<MetricsQuery>,
) -> HttpResult<impl IntoResponse> {
    let hours = query.hours.unwrap_or(168); // 7 days
    let metrics = ServerMetricsRepo::get_history(&storage.db_pool, hours).await?;

    Ok(Json(serde_json::json!({ "metrics": metrics })))
}
