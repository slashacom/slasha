use std::collections::HashSet;

use axum::{
    extract::{Path, State},
    response::IntoResponse,
    routing::{get, put},
    Json, Router,
};
use chrono::Utc;
use diesel::prelude::*;
use serde::Deserialize;
use uuid::Uuid;

use crate::{
    error::{Error, Result},
    extractors::auth::AuthUser,
    AppState,
};

use super::utils::{lookup_app_for_user, lookup_service_for_app};

use models::{schema::service_env_vars, service::ServiceEnvVar};

pub fn router() -> Router<AppState> {
    Router::new()
        .route("/", get(get_env_vars))
        .route("/", put(update_env_vars))
}

#[derive(Deserialize)]
struct EnvVarItem {
    key: String,
    value: String,
}

#[derive(Deserialize)]
struct UpdateEnvVarsReq {
    vars: Vec<EnvVarItem>,
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

    Ok(Json(serde_json::json!({
        "env_vars": vars,
    })))
}

async fn update_env_vars(
    State(state): State<AppState>,
    auth: AuthUser,
    Path((slug, service_id)): Path<(String, String)>,
    Json(payload): Json<UpdateEnvVarsReq>,
) -> Result<impl IntoResponse> {
    let app = lookup_app_for_user(&state, &slug, &auth.0.id)?;

    let mut seen_keys = HashSet::new();
    for item in &payload.vars {
        if !seen_keys.insert(&item.key) {
            return Err(Error::BadRequest(format!(
                "Duplicate key found in request: {}",
                item.key
            )));
        }
    }

    lookup_service_for_app(&state, &app.id, &service_id)?;

    let mut conn = state
        .db_pool
        .get()
        .map_err(|e| Error::Internal(anyhow::anyhow!("DB pool error: {}", e)))?;

    let now = Utc::now().naive_utc();
    let new_vars: Vec<ServiceEnvVar> = payload
        .vars
        .into_iter()
        .map(|item| ServiceEnvVar {
            id: Uuid::new_v4().to_string(),
            service_id: service_id.clone(),
            key: item.key,
            value: item.value,
            created_at: now,
            updated_at: now,
        })
        .collect();

    conn.transaction::<_, diesel::result::Error, _>(|conn| {
        diesel::delete(service_env_vars::table.filter(service_env_vars::service_id.eq(&service_id)))
            .execute(conn)?;

        if !new_vars.is_empty() {
            diesel::insert_into(service_env_vars::table)
                .values(&new_vars)
                .execute(conn)?;
        }

        Ok(())
    })?;

    Ok(Json(serde_json::json!({
        "env_vars": new_vars,
    })))
}
