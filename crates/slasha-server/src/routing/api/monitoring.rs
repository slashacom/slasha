use axum::{
    Json, Router,
    extract::{Query, State},
    response::IntoResponse,
    routing::get,
};
use serde::Deserialize;
use slasha_db::{models::node::LOCAL_NODE_ID, repos::server_metrics::ServerMetricsRepo};

use crate::{AppState, HttpResult, extractors::auth::AuthUser, state::Storage};

#[derive(Deserialize)]
pub struct MetricsQuery {
    pub node_id: Option<String>,
    pub start: Option<chrono::DateTime<chrono::Utc>>,
    pub end: Option<chrono::DateTime<chrono::Utc>>,
}

pub fn router() -> Router<AppState> {
    Router::new().route("/metrics", get(get_metrics))
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

    let node_id = query.node_id.unwrap_or_else(|| LOCAL_NODE_ID.to_string());

    let metrics = ServerMetricsRepo::get_history(
        &storage.duckdb_pool,
        &node_id,
        start.naive_utc(),
        end.naive_utc(),
    )
    .await?;

    Ok(Json(serde_json::json!({ "metrics": metrics })))
}
