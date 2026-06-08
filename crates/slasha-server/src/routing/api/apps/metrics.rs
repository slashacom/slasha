use axum::{
    Json,
    extract::{Path, Query, State},
    response::IntoResponse,
};
use serde::Deserialize;
use slasha_db::repos::{app::AppRepo, app_metrics::AppMetricsRepo};

use crate::{error::HttpResult, extractors::auth::AuthUser, state::Storage};

#[derive(Deserialize)]
pub struct MetricsQuery {
    pub hours: Option<i64>,
}

pub async fn get_metrics(
    State(storage): State<Storage>,
    AuthUser(user): AuthUser,
    Path(slug): Path<String>,
    Query(query): Query<MetricsQuery>,
) -> HttpResult<impl IntoResponse> {
    let app = AppRepo::find_by_slug_for_user(&storage.db_pool, &slug, &user.id).await?;

    let hours = query.hours.unwrap_or(168); // 7 days
    let metrics = AppMetricsRepo::get_history(&storage.db_pool, &app.id, hours).await?;

    Ok(Json(serde_json::json!({ "metrics": metrics })))
}
