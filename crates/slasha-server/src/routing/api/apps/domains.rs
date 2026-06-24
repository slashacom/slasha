use axum::{
    Json, Router,
    extract::{Path, State},
    response::IntoResponse,
    routing::{delete, get},
};
use serde::Deserialize;
use slasha_db::repos::app_domain::AppDomainRepo;

use crate::{AppState, error::HttpResult, extractors::app::ActiveApp, state::Storage};

pub fn router() -> Router<AppState> {
    Router::new()
        .route("/", get(list_domains).post(add_domain))
        .route("/{domain_id}", delete(delete_domain))
}

#[derive(Deserialize)]
struct AddDomainRequest {
    domain: String,
}

async fn list_domains(
    State(storage): State<Storage>,
    ActiveApp { app, .. }: ActiveApp,
) -> HttpResult<impl IntoResponse> {
    let domains = AppDomainRepo::list_for_app(&storage.db_pool, &app.id).await?;

    Ok(Json(serde_json::json!({ "domains": domains })))
}

async fn add_domain(
    State(state): State<AppState>,
    ActiveApp { app, .. }: ActiveApp,
    Json(payload): Json<AddDomainRequest>,
) -> HttpResult<impl IntoResponse> {
    let domain = AppDomainRepo::add(&state.storage.db_pool, &app.id, &payload.domain).await?;

    state.runtime.proxy_sync_trigger.notify_one();

    Ok(Json(serde_json::json!({ "domain": domain })))
}

async fn delete_domain(
    State(state): State<AppState>,
    ActiveApp { .. }: ActiveApp,
    Path((_, domain_id)): Path<(String, String)>,
) -> HttpResult<impl IntoResponse> {
    AppDomainRepo::delete(&state.storage.db_pool, &domain_id).await?;

    state.runtime.proxy_sync_trigger.notify_one();

    Ok(Json(serde_json::json!({ "ok": true })))
}
