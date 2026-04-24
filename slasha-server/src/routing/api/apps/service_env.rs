use axum::{
    Json, Router,
    extract::{Path, State},
    response::IntoResponse,
    routing::{get, put},
};
use chrono::Utc;
use diesel::prelude::*;
use serde::Deserialize;
use uuid::Uuid;

use crate::{
    AppState,
    error::{Error, Result},
    extractors::auth::AuthUser,
};

use super::utils::{lookup_app_for_user, lookup_service_for_app};

use models::{schema::service_env_vars, service::ServiceEnvVar};

pub fn router() -> Router<AppState> {
    Router::new()
        .route("/", get(get_env_vars))
        .route("/", put(update_env_vars))
}

#[derive(Deserialize)]
struct UpdateEnvVarsReq {
    vars: std::collections::HashMap<String, String>,
}

async fn get_env_vars(
    State(state): State<AppState>,
    auth: AuthUser,
    Path((slug, service_id)): Path<(String, String)>,
) -> Result<impl IntoResponse> {
    let app = lookup_app_for_user(&state, &slug, &auth.0.id)?;

    lookup_service_for_app(&state, &app.id, &service_id)?;

    let mut conn = state
        .db_pool
        .get()
        .map_err(|e| Error::Internal(anyhow::anyhow!("DB pool error: {}", e)))?;

    let vars: Vec<ServiceEnvVar> = service_env_vars::table
        .filter(service_env_vars::service_id.eq(&service_id))
        .order(service_env_vars::key.asc())
        .load(&mut conn)?;

    let env_map: std::collections::HashMap<String, String> =
        vars.into_iter().map(|v| (v.key, v.value)).collect();

    Ok(Json(serde_json::json!({
        "env_vars": env_map,
    })))
}

async fn update_env_vars(
    State(state): State<AppState>,
    auth: AuthUser,
    Path((slug, service_id)): Path<(String, String)>,
    Json(payload): Json<UpdateEnvVarsReq>,
) -> Result<impl IntoResponse> {
    let app = lookup_app_for_user(&state, &slug, &auth.0.id)?;

    lookup_service_for_app(&state, &app.id, &service_id)?;

    let mut conn = state
        .db_pool
        .get()
        .map_err(|e| Error::Internal(anyhow::anyhow!("DB pool error: {}", e)))?;

    let now = Utc::now().naive_utc();
    let new_vars: Vec<ServiceEnvVar> = payload
        .vars
        .into_iter()
        .map(|(key, value)| ServiceEnvVar {
            id: Uuid::new_v4().to_string(),
            service_id: service_id.clone(),
            key,
            value,
            created_at: now,
            updated_at: now,
        })
        .collect();

    conn.transaction::<_, diesel::result::Error, _>(|conn| {
        diesel::delete(
            service_env_vars::table.filter(service_env_vars::service_id.eq(&service_id)),
        )
        .execute(conn)?;

        if !new_vars.is_empty() {
            diesel::insert_into(service_env_vars::table)
                .values(&new_vars)
                .execute(conn)?;
        }

        Ok(())
    })?;

    let env_map: std::collections::HashMap<String, String> = new_vars
        .iter()
        .map(|v| (v.key.clone(), v.value.clone()))
        .collect();

    Ok(Json(serde_json::json!({
        "env_vars": env_map,
    })))
}
