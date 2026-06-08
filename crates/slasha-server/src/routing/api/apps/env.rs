use std::collections::HashMap;

use axum::{
    Json, Router,
    extract::{Path, State},
    response::IntoResponse,
    routing::{get, put},
};
use chrono::Utc;
use serde::{Deserialize, Serialize};
use slasha_db::{
    app::AppEnvVar,
    repos::{app::AppRepo, service::ServiceRepo},
};
use uuid::Uuid;

use crate::{
    error::HttpResult,
    extractors::auth::AuthUser,
    state::{AppState, Storage},
};

pub fn router() -> Router<AppState> {
    Router::new()
        .route("/", get(get_env_vars))
        .route("/", put(update_env_vars))
        .route("/suggestions", get(get_env_suggestions))
}

#[derive(Deserialize)]
struct UpdateEnvVarsReq {
    vars: HashMap<String, String>,
}

async fn get_env_vars(
    State(storage): State<Storage>,
    AuthUser(user): AuthUser,
    Path(slug): Path<String>,
) -> HttpResult<impl IntoResponse> {
    let app = AppRepo::find_by_slug_for_user(&storage.db_pool, &slug, &user.id).await?;

    let vars = AppRepo::get_env_vars(&storage.db_pool, &app.id).await?;

    let env_map: HashMap<String, String> = vars.into_iter().map(|v| (v.key, v.value)).collect();

    Ok(Json(serde_json::json!({
        "env_vars": env_map,
    })))
}

#[derive(Serialize)]
struct ServiceSuggestion {
    name: String,
    env_keys: Vec<String>,
}

async fn get_env_suggestions(
    State(storage): State<Storage>,
    AuthUser(user): AuthUser,
    Path(slug): Path<String>,
) -> HttpResult<impl IntoResponse> {
    let app = AppRepo::find_by_slug_for_user(&storage.db_pool, &slug, &user.id).await?;
    let services = ServiceRepo::list_for_app(&storage.db_pool, &app.id).await?;

    let mut out: Vec<ServiceSuggestion> = Vec::with_capacity(services.len());
    for svc in services {
        let vars = ServiceRepo::get_env_vars(&storage.db_pool, &svc.id).await?;
        let mut env_keys: Vec<String> = vars.into_iter().map(|v| v.key).collect();
        env_keys.sort();
        env_keys.insert(0, "service_container_name".to_string());
        out.push(ServiceSuggestion {
            name: svc.name,
            env_keys,
        });
    }

    Ok(Json(serde_json::json!({
        "services": out,
    })))
}

async fn update_env_vars(
    State(storage): State<Storage>,
    AuthUser(user): AuthUser,
    Path(slug): Path<String>,
    Json(payload): Json<UpdateEnvVarsReq>,
) -> HttpResult<impl IntoResponse> {
    let app = AppRepo::find_by_slug_for_user(&storage.db_pool, &slug, &user.id).await?;

    let now = Utc::now().naive_utc();
    let new_vars: Vec<AppEnvVar> = payload
        .vars
        .into_iter()
        .map(|(key, value)| AppEnvVar {
            id: Uuid::new_v4().to_string(),
            app_id: app.id.clone(),
            key,
            value,
            created_at: now,
            updated_at: now,
        })
        .collect();

    let new_vars = AppRepo::set_env_vars(&storage.db_pool, &app.id, new_vars).await?;

    let env_map: std::collections::HashMap<String, String> = new_vars
        .iter()
        .map(|v| (v.key.clone(), v.value.clone()))
        .collect();

    Ok(Json(serde_json::json!({
        "env_vars": env_map,
    })))
}
