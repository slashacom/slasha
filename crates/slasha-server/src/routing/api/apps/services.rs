use std::collections::HashMap;

use axum::{
    Json, Router,
    body::Body,
    extract::{Path, Query, State, WebSocketUpgrade},
    http::header,
    response::{IntoResponse, Response},
    routing::{get, post},
};
use chrono::Utc;
use garde::Validate;
use serde::Deserialize;
use slasha_db::{
    DbPool, DuckdbPool,
    repos::{node::NodeRepo, service::ServiceRepo},
    service::{NewServiceEnvVar, ServiceKind, ServiceResources, ServiceStatus},
};

use crate::{
    HttpError, HttpResult,
    docker::service::ServiceDocker,
    extractors::{
        ValidatedJson,
        app::{ActiveApp, ActiveAppOwner},
    },
    logs::LogBus,
    routing::api::{
        logs::{LogQuery, fetch_resource_logs, stream_resource_logs},
        validation::not_empty,
    },
    state::AppState,
    tunnel,
};

pub fn router() -> Router<AppState> {
    Router::new()
        .route("/", get(list_services))
        .route("/", post(create_service))
        .route("/{id}/env", get(get_env_vars).put(update_env_vars))
        .route("/{id}/logs", get(get_logs))
        .route("/{id}/stream", get(stream_logs))
        .route("/{id}/backup", get(backup_service))
        .route("/{id}/tunnel", get(tunnel))
        .route("/{id}/restart", post(restart_service))
        .route("/{id}/redeploy", post(redeploy_service))
        .route("/{id}/stop", post(stop_service))
        .route("/{id}/stats", get(service_stats))
        .route("/{id}", get(get_service).delete(delete_service))
}

#[derive(Deserialize, Validate)]
struct CreateServiceReq {
    #[garde(skip)]
    kind: ServiceKind,
    #[serde(deserialize_with = "crate::routing::api::deserialize::trim_string")]
    #[garde(custom(not_empty))]
    name: String,
    #[garde(custom(not_empty))]
    version: String,
    #[garde(skip)]
    env_vars: HashMap<String, String>,
    #[serde(default)]
    #[garde(skip)]
    resources: Option<ServiceResources>,
}

async fn list_services(
    State(db_pool): State<DbPool>,
    ActiveApp { app, .. }: ActiveApp,
) -> HttpResult<impl IntoResponse> {
    let services = ServiceRepo::list_for_app(&db_pool, &app.id).await?;

    Ok(Json(serde_json::json!({
        "services": services,
    })))
}

async fn get_service(
    State(db_pool): State<DbPool>,
    ActiveApp { app, .. }: ActiveApp,
    Path((_, id)): Path<(String, String)>,
) -> HttpResult<impl IntoResponse> {
    let service = ServiceRepo::find(&db_pool, &id, &app.id).await?;

    Ok(Json(serde_json::json!({ "service": service })))
}

async fn service_stats(
    State(state): State<AppState>,
    ActiveApp { app, .. }: ActiveApp,
    Path((_, id)): Path<(String, String)>,
) -> HttpResult<impl IntoResponse> {
    let stats = ServiceDocker::new(state, app).await?.get_stats(&id).await?;

    Ok(Json(stats))
}

async fn create_service(
    State(state): State<AppState>,
    ActiveAppOwner { app, .. }: ActiveAppOwner,
    ValidatedJson(payload): ValidatedJson<CreateServiceReq>,
) -> HttpResult<impl IntoResponse> {
    if payload.env_vars.contains_key("DATABASE_URL") {
        return Err(HttpError::bad_request(
            "DATABASE_URL cannot be set manually as it is automatically managed and exported by Slasha",
        ));
    }

    let service = ServiceDocker::new(state, app)
        .await?
        .provision(
            payload.kind,
            payload.name,
            payload.version,
            payload.env_vars,
            payload.resources,
        )
        .await?;

    Ok(Json(serde_json::json!({ "service": service })))
}

async fn tunnel(
    ws: WebSocketUpgrade,
    State(state): State<AppState>,
    ActiveAppOwner { app, user, .. }: ActiveAppOwner,
    Path((_, id)): Path<(String, String)>,
) -> HttpResult<impl IntoResponse> {
    let service = ServiceRepo::find(&state.storage.db_pool, &id, &app.id).await?;

    if service.status != ServiceStatus::Running {
        return Err(HttpError::bad_request("Service is not running"));
    }

    let node = NodeRepo::get(&state.storage.db_pool, &app.node_id).await?;
    let docker_client = state.clients.docker_registry.get_client(&node)?;

    Ok(ws.on_upgrade(move |socket| async move {
        tunnel::handle_tunnel(
            socket,
            docker_client,
            state.storage.db_pool,
            service,
            user.id,
        )
        .await;
    }))
}

async fn restart_service(
    State(state): State<AppState>,
    ActiveAppOwner { app, .. }: ActiveAppOwner,
    Path((_, id)): Path<(String, String)>,
) -> HttpResult<impl IntoResponse> {
    ServiceDocker::new(state, app)
        .await?
        .restart_service(&id)
        .await?;

    Ok(Json(serde_json::json!({ "restarted": true })))
}

async fn redeploy_service(
    State(state): State<AppState>,
    ActiveAppOwner { app, .. }: ActiveAppOwner,
    Path((_, id)): Path<(String, String)>,
) -> HttpResult<impl IntoResponse> {
    ServiceDocker::new(state, app)
        .await?
        .redeploy_service(&id)
        .await?;

    Ok(Json(serde_json::json!({ "redeploying": true })))
}

async fn stop_service(
    State(state): State<AppState>,
    ActiveAppOwner { app, .. }: ActiveAppOwner,
    Path((_, id)): Path<(String, String)>,
) -> HttpResult<impl IntoResponse> {
    ServiceDocker::new(state, app)
        .await?
        .stop_service(&id)
        .await?;

    Ok(Json(serde_json::json!({ "stopped": true })))
}

async fn delete_service(
    State(state): State<AppState>,
    ActiveAppOwner { app, .. }: ActiveAppOwner,
    Path((_, id)): Path<(String, String)>,
) -> HttpResult<impl IntoResponse> {
    ServiceDocker::new(state, app)
        .await?
        .delete_service(&id)
        .await?;

    Ok(Json(serde_json::json!({ "deleted": true })))
}

async fn backup_service(
    State(state): State<AppState>,
    ActiveAppOwner { app, .. }: ActiveAppOwner,
    Path((_, id)): Path<(String, String)>,
) -> HttpResult<impl IntoResponse> {
    let service = ServiceRepo::find(&state.storage.db_pool, &id, &app.id).await?;

    let byte_stream = ServiceDocker::new(state.clone(), app.clone())
        .await?
        .backup_service(&id)
        .await?;

    let timestamp = Utc::now().format("%Y%m%d%H%M%S");
    let filename = format!("{}-{}.dump", service.name, timestamp);

    let response = Response::builder()
        .header(header::CONTENT_TYPE, "application/octet-stream")
        .header(
            header::CONTENT_DISPOSITION,
            format!("attachment; filename=\"{}\"", filename),
        )
        .body(Body::from_stream(byte_stream))
        .map_err(|e| HttpError::internal(anyhow::anyhow!(e)))?;

    Ok(response)
}

async fn get_logs(
    State(db_pool): State<DbPool>,
    State(duckdb_pool): State<DuckdbPool>,
    ActiveApp { app, .. }: ActiveApp,
    Path((_, service_id)): Path<(String, String)>,
    Query(query): Query<LogQuery>,
) -> HttpResult<impl IntoResponse> {
    ServiceRepo::find(&db_pool, &service_id, &app.id).await?;
    fetch_resource_logs(&duckdb_pool, &service_id, query).await
}

async fn stream_logs(
    State(db_pool): State<DbPool>,
    State(log_bus): State<LogBus>,
    ActiveApp { app, .. }: ActiveApp,
    Path((_, service_id)): Path<(String, String)>,
) -> HttpResult<impl IntoResponse> {
    ServiceRepo::find(&db_pool, &service_id, &app.id).await?;
    stream_resource_logs(&log_bus, &service_id).await
}

#[derive(Deserialize, Validate)]
struct UpdateEnvVarsReq {
    #[garde(skip)]
    vars: HashMap<String, String>,
}

async fn get_env_vars(
    State(db_pool): State<DbPool>,
    ActiveApp { app, .. }: ActiveApp,
    Path((_, service_id)): Path<(String, String)>,
) -> HttpResult<impl IntoResponse> {
    ServiceRepo::find(&db_pool, &service_id, &app.id).await?;

    let vars = ServiceRepo::get_env_vars(&db_pool, &service_id).await?;

    let env_map: HashMap<String, String> = vars.into_iter().map(|v| (v.key, v.value)).collect();

    Ok(Json(serde_json::json!({
        "env_vars": env_map,
    })))
}

async fn update_env_vars(
    State(db_pool): State<DbPool>,
    ActiveAppOwner { app, .. }: ActiveAppOwner,
    Path((_, service_id)): Path<(String, String)>,
    ValidatedJson(payload): ValidatedJson<UpdateEnvVarsReq>,
) -> HttpResult<impl IntoResponse> {
    ServiceRepo::find(&db_pool, &service_id, &app.id).await?;

    for (key, val) in &payload.vars {
        if key == "DATABASE_URL" {
            return Err(HttpError::bad_request(
                "DATABASE_URL cannot be set manually as it is automatically managed and exported by Slasha",
            ));
        }

        if val.trim().is_empty() {
            return Err(HttpError::bad_request(format!(
                "Environment variable '{}' cannot be empty",
                key
            )));
        }
    }

    let new_vars: Vec<NewServiceEnvVar> = payload
        .vars
        .into_iter()
        .map(|(key, value)| NewServiceEnvVar {
            service_id: service_id.clone(),
            key,
            value,
        })
        .collect();

    let new_vars = ServiceRepo::set_env_vars(&db_pool, &service_id, new_vars).await?;

    Ok(Json(serde_json::json!({
        "env_vars": new_vars.into_iter().map(|v| (v.key, v.value)).collect::<HashMap<String, String>>(),
    })))
}
