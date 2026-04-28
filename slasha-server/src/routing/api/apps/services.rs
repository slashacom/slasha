use std::collections::HashMap;

use axum::{
    Json, Router,
    extract::{Path, State},
    response::{
        IntoResponse,
        sse::{Event, KeepAlive, Sse},
    },
    routing::{delete, get, post},
};
use chrono::Utc;
use diesel::prelude::*;
use futures_util::{StreamExt, stream};
use serde::Deserialize;
use tokio_stream::wrappers::BroadcastStream;
use uuid::Uuid;

use crate::{
    docker::{
        logs::LogKey,
        services::{delete_service, provision_service, stop_service},
    },
    error::{Error, Result},
    extractors::auth::AuthUser,
    state::{AppState, Clients, Runtime, Storage},
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
        .route("/{id}/logs", get(stream_logs))
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
    State(storage): State<Storage>,
    AuthUser(user): AuthUser,
    Path(slug): Path<String>,
) -> Result<impl IntoResponse> {
    let app = lookup_app_for_user(&storage, &slug, &user.id)?;
    let mut conn = storage.db_pool.get()?;

    let app_services: Vec<Service> = services::table
        .filter(services::app_id.eq(&app.id))
        .order(services::created_at.desc())
        .load(&mut conn)?;

    Ok(Json(serde_json::json!({
        "services": app_services,
    })))
}

async fn create_service(
    State(clients): State<Clients>,
    State(storage): State<Storage>,
    State(runtime): State<Runtime>,
    AuthUser(user): AuthUser,
    Path(slug): Path<String>,
    Json(payload): Json<CreateServiceReq>,
) -> Result<impl IntoResponse> {
    let app = lookup_app_for_user(&storage, &slug, &user.id)?;

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

    let mut conn = storage.db_pool.get()?;

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

    let clients_clone = clients.clone();
    let storage_clone = storage.clone();
    let runtime_clone = runtime.clone();
    let app_clone = app.clone();
    let service_clone = new_service.clone();
    let env_vars_clone = payload.env_vars.clone();

    tokio::spawn(async move {
        if let Err(e) = provision_service(
            &clients_clone.docker,
            &storage_clone.db_pool,
            &runtime_clone.log_manager,
            &app_clone,
            &service_clone,
            env_vars_clone,
        )
        .await
        {
            tracing::error!("Failed to provision service {}: {}", service_clone.id, e);
            if let Ok(mut conn) = storage_clone.db_pool.get() {
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
    State(clients): State<Clients>,
    State(storage): State<Storage>,
    State(runtime): State<Runtime>,
    AuthUser(user): AuthUser,
    Path((slug, id)): Path<(String, String)>,
) -> Result<impl IntoResponse> {
    let app = lookup_app_for_user(&storage, &slug, &user.id)?;
    let mut conn = storage.db_pool.get()?;

    let svc = services::table
        .filter(services::id.eq(&id))
        .filter(services::app_id.eq(&app.id))
        .first::<Service>(&mut conn)
        .optional()?
        .ok_or_else(|| Error::NotFound("Service not found".into()))?;

    if svc.status != ServiceStatus::Running {
        return Err(Error::BadRequest("Service is not running".into()));
    }

    stop_service(
        &clients.docker,
        &storage.db_pool,
        &runtime.log_manager,
        &app,
        &svc,
    )
    .await
    .map_err(|e| Error::Internal(anyhow::anyhow!("Failed to stop service: {}", e)))?;

    Ok(Json(serde_json::json!({ "stopped": true })))
}

async fn delete_service_handler(
    State(clients): State<Clients>,
    State(storage): State<Storage>,
    State(runtime): State<Runtime>,
    AuthUser(user): AuthUser,
    Path((slug, id)): Path<(String, String)>,
) -> Result<impl IntoResponse> {
    let app = lookup_app_for_user(&storage, &slug, &user.id)?;
    let mut conn = storage.db_pool.get()?;

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

    delete_service(
        &clients.docker,
        &storage.db_pool,
        &runtime.log_manager,
        &app,
        &svc,
    )
    .await
    .map_err(|e| Error::Internal(anyhow::anyhow!("Failed to delete service: {}", e)))?;

    Ok(Json(serde_json::json!({ "deleted": true })))
}

async fn stream_logs(
    State(storage): State<Storage>,
    State(runtime): State<Runtime>,
    AuthUser(user): AuthUser,
    Path((slug, id)): Path<(String, String)>,
) -> Result<
    Sse<impl futures_util::Stream<Item = std::result::Result<Event, std::convert::Infallible>>>,
> {
    let app = lookup_app_for_user(&storage, &slug, &user.id)?;

    let mut conn = storage.db_pool.get()?;

    let svc = services::table
        .filter(services::id.eq(&id))
        .filter(services::app_id.eq(&app.id))
        .first::<Service>(&mut conn)
        .optional()?
        .ok_or_else(|| Error::NotFound(format!("Service '{}' not found", id)))?;

    let log = runtime
        .log_manager
        .get_logger(&LogKey::Service {
            app_slug: app.slug.clone(),
            service_name: svc.name.clone(),
        })
        .await
        .map_err(|e| Error::Internal(anyhow::anyhow!("Failed to get logger: {}", e)))?;

    let historical = log.get_historical().await?;

    let historical_stream = stream::iter(
        historical
            .into_iter()
            .map(|msg| Ok(Event::default().data(msg))),
    );

    let rx = log.subscribe();
    let live_stream = BroadcastStream::new(rx).map(|res| match res {
        Ok(msg) => Ok(Event::default().data(msg)),
        Err(e) => Ok(Event::default().event("error").data(e.to_string())),
    });

    let combined = historical_stream.chain(live_stream);

    Ok(Sse::new(combined).keep_alive(KeepAlive::default()))
}
