use axum::{
    Json, Router,
    extract::{Path, State},
    response::IntoResponse,
    routing::{delete, get},
};
use garde::Validate;
use serde::Deserialize;
use slasha_db::repos::app_domain::AppDomainRepo;

use crate::{
    AppState, HttpResult, domain_health,
    extractors::{ValidatedJson, app::ActiveApp},
    routing::api::validation::not_empty,
    state::Storage,
};

pub fn router() -> Router<AppState> {
    Router::new()
        .route("/", get(list_domains).post(add_domain))
        .route("/health", get(list_domains_health))
        .route("/{domain_id}", delete(delete_domain))
}

#[derive(Deserialize, Validate)]
struct AddDomainRequest {
    #[serde(deserialize_with = "crate::routing::api::deserialize::trim_string")]
    #[garde(custom(not_empty))]
    domain: String,
}

async fn list_domains(
    State(storage): State<Storage>,
    ActiveApp { app, .. }: ActiveApp,
) -> HttpResult<impl IntoResponse> {
    let domains = AppDomainRepo::list_for_app(&storage.db_pool, &app.id).await?;

    Ok(Json(serde_json::json!({ "domains": domains })))
}

async fn list_domains_health(
    State(state): State<AppState>,
    ActiveApp { app, .. }: ActiveApp,
) -> HttpResult<impl IntoResponse> {
    let domains = AppDomainRepo::list_for_app(&state.storage.db_pool, &app.id).await?;
    let names = domains.into_iter().map(|d| d.domain).collect();

    let health = domain_health::check_domains(names, &state.config).await;

    Ok(Json(serde_json::json!({ "health": health })))
}

async fn add_domain(
    State(state): State<AppState>,
    ActiveApp { app, .. }: ActiveApp,
    ValidatedJson(payload): ValidatedJson<AddDomainRequest>,
) -> HttpResult<impl IntoResponse> {
    let raw_domain = payload.domain.as_str();

    let url_str = if !raw_domain.starts_with("http://") && !raw_domain.starts_with("https://") {
        format!("https://{}", raw_domain)
    } else {
        raw_domain.to_string()
    };

    let cleaned = match reqwest::Url::parse(&url_str) {
        Ok(parsed) => {
            if let Some(host) = parsed.host_str() {
                host.to_string()
            } else {
                raw_domain.trim_end_matches('/').to_string()
            }
        }
        Err(_) => raw_domain.trim_end_matches('/').to_string(),
    };

    let domain = AppDomainRepo::add(
        &state.storage.db_pool,
        slasha_db::app::NewAppDomain {
            app_id: app.id.clone(),
            domain: cleaned,
        },
    )
    .await?;

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
