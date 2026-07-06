use std::collections::HashMap;

use axum::{
    Json, Router,
    extract::{Path, State},
    response::IntoResponse,
    routing::{get, put},
};
use garde::Validate;
use serde::Deserialize;
use slasha_db::{repos::service::ServiceRepo, service::NewServiceEnvVar};

use crate::{
    HttpResult,
    extractors::{ValidatedJson, app::ActiveApp},
    state::{AppState, Storage},
};

pub fn router() -> Router<AppState> {
    Router::new()
        .route("/", get(get_env_vars))
        .route("/", put(update_env_vars))
}

#[derive(Deserialize, Validate)]
struct UpdateEnvVarsReq {
    #[garde(skip)]
    vars: std::collections::HashMap<String, String>,
}

async fn get_env_vars(
    State(storage): State<Storage>,
    ActiveApp { app, .. }: ActiveApp,
    Path((_, service_id)): Path<(String, String)>,
) -> HttpResult<impl IntoResponse> {
    ServiceRepo::find(&storage.db_pool, &service_id, &app.id).await?;

    let vars = ServiceRepo::get_env_vars(&storage.db_pool, &service_id).await?;

    let env_map: std::collections::HashMap<String, String> =
        vars.into_iter().map(|v| (v.key, v.value)).collect();

    Ok(Json(serde_json::json!({
        "env_vars": env_map,
    })))
}

async fn update_env_vars(
    State(storage): State<Storage>,
    ActiveApp { app, .. }: ActiveApp,
    Path((_, service_id)): Path<(String, String)>,
    ValidatedJson(payload): ValidatedJson<UpdateEnvVarsReq>,
) -> HttpResult<impl IntoResponse> {
    ServiceRepo::find(&storage.db_pool, &service_id, &app.id).await?;

    let new_vars: Vec<NewServiceEnvVar> = payload
        .vars
        .into_iter()
        .map(|(key, value)| NewServiceEnvVar {
            service_id: service_id.clone(),
            key,
            value,
        })
        .collect();

    let new_vars = ServiceRepo::set_env_vars(&storage.db_pool, &service_id, new_vars).await?;

    Ok(Json(serde_json::json!({
        "env_vars": new_vars.into_iter().map(|v| (v.key, v.value)).collect::<HashMap<String, String>>(),
    })))
}
