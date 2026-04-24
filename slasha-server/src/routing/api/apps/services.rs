use std::collections::HashMap;

use axum::{
    Json, Router,
    extract::{Path, State},
    response::IntoResponse,
    routing::{delete, get, post},
};
use chrono::Utc;
use diesel::prelude::*;
use serde::Deserialize;
use uuid::Uuid;

use crate::{
    AppState,
    docker::services::{delete_service, provision_service, stop_service},
    error::{Error, Result},
    extractors::auth::AuthUser,
};

use super::utils::lookup_app_for_user;

use models::{
    schema::services,
    service::{Service, ServiceKind, ServiceStatus},
};

pub fn router() -> Router<AppState> {
    Router::new()
        .route("/", get(list_services))
        .route("/", post(create_service))
        .route("/{id}/stop", post(stop_service_handler))
        .route("/{id}", delete(delete_service_handler))
}

#[derive(Deserialize)]
struct CreateServiceReq {
    kind: ServiceKind,
    name: String,
    version: String,
    env_vars: HashMap<String, String>,
}

async fn list_services(
    State(state): State<AppState>,
    auth: AuthUser,
    Path(slug): Path<String>,
) -> Result<impl IntoResponse> {
    let app = lookup_app_for_user(&state, &slug, &auth.0.id)?;
    let mut conn = state
        .db_pool
        .get()
        ?;

    let app_services: Vec<Service> = services::table
        .filter(services::app_id.eq(&app.id))
        .order(services::created_at.desc())
        .load(&mut conn)?;

    Ok(Json(serde_json::json!({
        "services": app_services,
    })))
}

async fn create_service(
    State(state): State<AppState>,
    auth: AuthUser,
    Path(slug): Path<String>,
    Json(payload): Json<CreateServiceReq>,
) -> Result<impl IntoResponse> {
    let app = lookup_app_for_user(&state, &slug, &auth.0.id)?;

    if !payload
        .kind
        .supported_versions()
        .contains(&payload.version.as_str())
    {
        return Err(Error::BadRequest(format!(
            "Version {} is not supported for {:?}",
            payload.version, payload.kind
        )));
    }

    let mut conn = state
        .db_pool
        .get()
        ?;

    let now = Utc::now().naive_utc();
    let service_id = Uuid::new_v4().to_string();

    let new_service = Service {
        id: service_id.clone(),
        app_id: app.id.clone(),
        kind: payload.kind,
        name: payload.name.trim().to_string(),
        version: payload.version,
        status: ServiceStatus::Provisioning,
        created_at: now,
        updated_at: now,
    };

    diesel::insert_into(services::table)
        .values(&new_service)
        .execute(&mut conn)?;

    let state_clone = state.clone();
    let app_clone = app.clone();
    let service_clone = new_service.clone();
    let env_vars_clone = payload.env_vars.clone();

    tokio::spawn(async move {
        if let Err(e) = provision_service(
            &state_clone.docker,
            &state_clone.db_pool,
            &app_clone,
            &service_clone,
            env_vars_clone,
        )
        .await
        {
            tracing::error!("Failed to provision service {}: {}", service_clone.id, e);
            if let Ok(mut conn) = state_clone.db_pool.get() {
                let _ = crate::docker::services::update_service_status(
                    &mut conn,
                    &service_clone.id,
                    ServiceStatus::Failed,
                );
            }
        }
    });

    Ok(Json(serde_json::json!({
        "service": new_service,
    })))
}

async fn stop_service_handler(
    State(state): State<AppState>,
    auth: AuthUser,
    Path((slug, id)): Path<(String, String)>,
) -> Result<impl IntoResponse> {
    let app = lookup_app_for_user(&state, &slug, &auth.0.id)?;
    let mut conn = state
        .db_pool
        .get()
        ?;

    let svc = services::table
        .filter(services::id.eq(&id))
        .filter(services::app_id.eq(&app.id))
        .first::<Service>(&mut conn)
        .optional()?
        .ok_or_else(|| Error::NotFound("Service not found".into()))?;

    if svc.status != ServiceStatus::Running {
        return Err(Error::BadRequest("Service is not running".into()));
    }

    stop_service(&state.docker, &state.db_pool, &svc)
        .await
        .map_err(|e| Error::Internal(anyhow::anyhow!("Failed to stop service: {}", e)))?;

    Ok(Json(serde_json::json!({ "stopped": true })))
}

async fn delete_service_handler(
    State(state): State<AppState>,
    auth: AuthUser,
    Path((slug, id)): Path<(String, String)>,
) -> Result<impl IntoResponse> {
    let app = lookup_app_for_user(&state, &slug, &auth.0.id)?;
    let mut conn = state
        .db_pool
        .get()
        ?;

    let svc = services::table
        .filter(services::id.eq(&id))
        .filter(services::app_id.eq(&app.id))
        .first::<Service>(&mut conn)
        .optional()?
        .ok_or_else(|| Error::NotFound("Service not found".into()))?;

    if svc.status != ServiceStatus::Stopped && svc.status != ServiceStatus::Failed {
        return Err(Error::BadRequest(
            "Cannot delete a running or provisioning service. Please stop it first.".into(),
        ));
    }

    delete_service(&state.docker, &state.db_pool, &svc)
        .await
        .map_err(|e| Error::Internal(anyhow::anyhow!("Failed to delete service: {}", e)))?;

    Ok(Json(serde_json::json!({ "deleted": true })))
}
