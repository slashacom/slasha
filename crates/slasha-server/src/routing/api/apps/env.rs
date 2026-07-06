use std::collections::HashMap;

use axum::{
    Json, Router,
    extract::State,
    response::IntoResponse,
    routing::{get, put},
};
use garde::Validate;
use serde::{Deserialize, Serialize};
use slasha_db::{
    app::NewAppEnvVar,
    repos::{app::AppRepo, service::ServiceRepo},
};

use crate::{
    HttpResult,
    extractors::{ValidatedJson, app::ActiveApp},
    state::{AppState, Storage},
};

pub fn router() -> Router<AppState> {
    Router::new()
        .route("/", get(get_env_vars))
        .route("/", put(update_env_vars))
        .route("/suggestions", get(get_env_suggestions))
}

#[derive(Deserialize, Validate)]
struct UpdateEnvVarsReq {
    #[garde(skip)]
    vars: HashMap<String, String>,
}

async fn get_env_vars(
    State(storage): State<Storage>,
    ActiveApp { app, .. }: ActiveApp,
) -> HttpResult<impl IntoResponse> {
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
    ActiveApp { app, .. }: ActiveApp,
) -> HttpResult<impl IntoResponse> {
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
    ActiveApp { app, .. }: ActiveApp,
    ValidatedJson(payload): ValidatedJson<UpdateEnvVarsReq>,
) -> HttpResult<impl IntoResponse> {
    let new_vars: Vec<NewAppEnvVar> = payload
        .vars
        .into_iter()
        .map(|(key, value)| NewAppEnvVar {
            app_id: app.id.clone(),
            key,
            value,
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
