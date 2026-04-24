use std::collections::HashMap;

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

use crate::{AppState, error::Result, extractors::auth::AuthUser};

use super::utils::lookup_app_for_user;

use models::{app::AppEnvVar, schema::app_env_vars};

pub fn router() -> Router<AppState> {
    Router::new()
        .route("/", get(get_env_vars))
        .route("/", put(update_env_vars))
}

#[derive(Deserialize)]
struct UpdateEnvVarsReq {
    vars: HashMap<String, String>,
}

async fn get_env_vars(
    State(state): State<AppState>,
    auth: AuthUser,
    Path(slug): Path<String>,
) -> Result<impl IntoResponse> {
    let app = lookup_app_for_user(&state, &slug, &auth.0.id)?;
    let mut conn = state.db_pool.get()?;

    let vars: Vec<AppEnvVar> = app_env_vars::table
        .filter(app_env_vars::app_id.eq(&app.id))
        .order(app_env_vars::key.asc())
        .load(&mut conn)?;

    let env_map: HashMap<String, String> = vars.into_iter().map(|v| (v.key, v.value)).collect();

    Ok(Json(serde_json::json!({
        "env_vars": env_map,
    })))
}

async fn update_env_vars(
    State(state): State<AppState>,
    auth: AuthUser,
    Path(slug): Path<String>,
    Json(payload): Json<UpdateEnvVarsReq>,
) -> Result<impl IntoResponse> {
    let app = lookup_app_for_user(&state, &slug, &auth.0.id)?;

    let mut conn = state.db_pool.get()?;

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

    let env_map: std::collections::HashMap<String, String> = new_vars
        .iter()
        .map(|v| (v.key.clone(), v.value.clone()))
        .collect();

    Ok(Json(serde_json::json!({
        "env_vars": env_map,
    })))
}
