use std::collections::HashSet;

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

use super::utils::lookup_app_for_user;

use models::{app::AppEnvVar, schema::app_env_vars};

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
    Path(slug): Path<String>,
) -> Result<impl IntoResponse> {
    let app = lookup_app_for_user(&state, &slug, &auth.0.id)?;
    let mut conn = state
        .db_pool
        .get()
        .map_err(|e| Error::Internal(anyhow::anyhow!("DB pool error: {}", e)))?;

    let vars: Vec<AppEnvVar> = app_env_vars::table
        .filter(app_env_vars::app_id.eq(&app.id))
        .order(app_env_vars::key.asc())
        .load(&mut conn)?;

    Ok(Json(serde_json::json!({
        "env_vars": vars,
    })))
}

async fn update_env_vars(
    State(state): State<AppState>,
    auth: AuthUser,
    Path(slug): Path<String>,
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

    let mut conn = state
        .db_pool
        .get()
        .map_err(|e| Error::Internal(anyhow::anyhow!("DB pool error: {}", e)))?;

    let now = Utc::now().naive_utc();
    let new_vars: Vec<AppEnvVar> = payload
        .vars
        .into_iter()
        .map(|item| AppEnvVar {
            id: Uuid::new_v4().to_string(),
            app_id: app.id.clone(),
            key: item.key,
            value: item.value,
            created_at: now,
            updated_at: now,
        })
        .collect();

    conn.transaction::<_, diesel::result::Error, _>(|conn| {
        diesel::delete(app_env_vars::table.filter(app_env_vars::app_id.eq(&app.id)))
            .execute(conn)?;

        if !new_vars.is_empty() {
            diesel::insert_into(app_env_vars::table)
                .values(&new_vars)
                .execute(conn)?;
        }

        Ok(())
    })?;

    Ok(Json(serde_json::json!({
        "env_vars": new_vars,
    })))
}
