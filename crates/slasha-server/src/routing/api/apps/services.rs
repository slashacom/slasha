use std::collections::HashMap;

use axum::{
    Json, Router,
    body::Body,
    extract::{Path, Query, State, WebSocketUpgrade},
    http::header,
    response::{IntoResponse, Response},
    routing::{delete, get, post},
};
use chrono::Utc;
use garde::Validate;
use serde::Deserialize;
use serde_json::json;
use slasha_db::{
    DbPool, DuckdbPool,
    models::service_backup::{NewServiceBackupConfig, ServiceBackupStatus, ServiceBackupTrigger},
    repos::{
        node::NodeRepo, s3_storage::S3StorageRepo, service::ServiceRepo,
        service_backup::ServiceBackupRepo,
    },
    service::{NewServiceEnvVar, Service, ServiceKind, ServiceResources, ServiceStatus},
};
use tokio::fs;
use tokio_util::io::ReaderStream;

use crate::{
    HttpError, HttpResult, cron,
    docker::service::{ServiceDocker, ServiceKindDockerExt, backup::service_backup_s3_key},
    extractors::{
        ValidatedJson,
        app::{ActiveApp, ActiveAppOwner},
    },
    logs::LogBus,
    operations::ResourceKey,
    routing::api::{
        logs::{LogQuery, fetch_resource_logs, stream_resource_logs},
        validation::{not_empty, valid_service_name},
    },
    s3,
    state::{AppState, Runtime},
    tunnel,
};

pub fn router() -> Router<AppState> {
    Router::new()
        .route("/", get(list_services))
        .route("/", post(create_service))
        .route("/{id}/env", get(get_env_vars).put(update_env_vars))
        .route("/{id}/logs", get(get_logs))
        .route("/{id}/stream", get(stream_logs))
        .route(
            "/{id}/backup-config",
            get(get_service_backup_config).put(update_service_backup_config),
        )
        .route(
            "/{id}/backups",
            get(list_service_backups).post(trigger_service_backup),
        )
        .route("/{id}/backups/{backup_id}", delete(delete_service_backup))
        .route(
            "/{id}/backups/{backup_id}/download",
            get(download_service_backup),
        )
        .route(
            "/{id}/backups/{backup_id}/restore",
            post(restore_service_backup),
        )
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
    #[garde(custom(valid_service_name))]
    name: String,
    #[garde(custom(not_empty))]
    version: String,
    #[garde(skip)]
    env_vars: HashMap<String, String>,
    #[serde(default)]
    #[garde(skip)]
    resources: Option<ServiceResources>,
}

fn derive_service_runtime_status(service: &Service, runtime: &Runtime) -> String {
    if let Some(status) = runtime
        .operations
        .status_of(&ResourceKey::service(&service.id))
    {
        return status.to_string();
    }

    service.status.to_string().to_lowercase()
}

async fn list_services(
    State(state): State<AppState>,
    ActiveApp { app, .. }: ActiveApp,
) -> HttpResult<impl IntoResponse> {
    let services = ServiceRepo::list_for_app(&state.storage.db_pool, &app.id).await?;
    let items: Vec<serde_json::Value> = services
        .into_iter()
        .map(|service| {
            let runtime_status = derive_service_runtime_status(&service, &state.runtime);
            serde_json::json!({
                "service": service,
                "runtime_status": runtime_status,
            })
        })
        .collect();

    Ok(Json(serde_json::json!({
        "services": items,
    })))
}

async fn get_service(
    State(state): State<AppState>,
    ActiveApp { app, .. }: ActiveApp,
    Path((_, id)): Path<(String, String)>,
) -> HttpResult<impl IntoResponse> {
    let service = ServiceRepo::find(&state.storage.db_pool, &id, &app.id).await?;
    let runtime_status = derive_service_runtime_status(&service, &state.runtime);

    Ok(Json(serde_json::json!({
        "service": service,
        "runtime_status": runtime_status,
    })))
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

    if ServiceRepo::name_exists(&state.storage.db_pool, &app.id, &payload.name).await? {
        return Err(HttpError::bad_request(format!(
            "Service with name '{}' already exists for this app",
            payload.name
        )));
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
    let docker_client = state.node_registry.get_client(&node)?;

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

async fn get_service_backup_config(
    State(state): State<AppState>,
    ActiveApp { app, .. }: ActiveApp,
    Path((_, id)): Path<(String, String)>,
) -> HttpResult<impl IntoResponse> {
    let _service = ServiceRepo::find(&state.storage.db_pool, &id, &app.id).await?;
    let config = ServiceBackupRepo::get_config(&state.storage.db_pool, &id).await?;
    Ok(Json(json!({ "config": config })))
}

#[derive(Deserialize, Validate)]
struct UpdateBackupConfigReq {
    #[garde(skip)]
    enabled: bool,
    #[garde(custom(not_empty))]
    schedule: String,
    #[garde(custom(not_empty))]
    timezone: String,
    #[garde(range(min = 1))]
    retention_count: i32,
    #[garde(skip)]
    s3_storage_id: Option<String>,
    #[garde(skip)]
    keep_local: Option<bool>,
}

async fn update_service_backup_config(
    State(state): State<AppState>,
    ActiveAppOwner { app, .. }: ActiveAppOwner,
    Path((_, id)): Path<(String, String)>,
    ValidatedJson(payload): ValidatedJson<UpdateBackupConfigReq>,
) -> HttpResult<impl IntoResponse> {
    let service = ServiceRepo::find(&state.storage.db_pool, &id, &app.id).await?;

    if !service.kind.supports_backups() {
        return Err(HttpError::bad_request(
            "Automated backups are not supported for this service kind",
        ));
    }

    let keep_local = payload.keep_local.unwrap_or(true);
    let s3_storage_id = payload.s3_storage_id.filter(|s| !s.is_empty());

    if payload.enabled && !keep_local && s3_storage_id.is_none() {
        return Err(HttpError::bad_request(
            "At least one backup destination (Local Storage or S3 Storage) must be enabled",
        ));
    }

    if let Some(ref s3_id) = s3_storage_id {
        S3StorageRepo::find(&state.storage.db_pool, s3_id).await?;
    }

    let calculated = cron::next_run_at(&payload.schedule, &payload.timezone, &Utc::now())
        .map_err(|e| HttpError::bad_request(e.to_string()))?;
    let next_run_at = payload.enabled.then_some(calculated);

    let new_config = NewServiceBackupConfig {
        service_id: id.clone(),
        enabled: payload.enabled,
        schedule: payload.schedule,
        timezone: payload.timezone,
        retention_count: payload.retention_count,
        s3_storage_id,
        keep_local,
        next_run_at,
    };

    let config = ServiceBackupRepo::upsert_config(&state.storage.db_pool, new_config).await?;
    Ok(Json(json!({ "config": config })))
}

async fn list_service_backups(
    State(state): State<AppState>,
    ActiveApp { app, .. }: ActiveApp,
    Path((_, id)): Path<(String, String)>,
) -> HttpResult<impl IntoResponse> {
    let _service = ServiceRepo::find(&state.storage.db_pool, &id, &app.id).await?;
    let backups = ServiceBackupRepo::list_backups(&state.storage.db_pool, &id, 100).await?;
    Ok(Json(json!({ "backups": backups })))
}

async fn trigger_service_backup(
    State(state): State<AppState>,
    ActiveAppOwner { app, .. }: ActiveAppOwner,
    Path((_, id)): Path<(String, String)>,
) -> HttpResult<impl IntoResponse> {
    let service_docker = ServiceDocker::new(state, app).await?;
    let backup = service_docker
        .run_service_backup(&id, ServiceBackupTrigger::Manual)
        .await?;
    Ok(Json(json!({ "backup": backup })))
}

async fn download_service_backup(
    State(state): State<AppState>,
    ActiveAppOwner { app, .. }: ActiveAppOwner,
    Path((_, id, backup_id)): Path<(String, String, String)>,
) -> HttpResult<impl IntoResponse> {
    let service = ServiceRepo::find(&state.storage.db_pool, &id, &app.id).await?;
    let backup = ServiceBackupRepo::find_backup(&state.storage.db_pool, &backup_id).await?;

    if backup.service_id != service.id {
        return Err(HttpError::not_found(format!(
            "Service backup \"{}\" not found",
            backup_id
        )));
    }

    if backup.status == ServiceBackupStatus::Running {
        return Err(HttpError::bad_request(
            "Service backup is currently running",
        ));
    }

    if backup.status != ServiceBackupStatus::Succeeded {
        return Err(HttpError::bad_request(
            "Cannot download an incomplete or failed backup",
        ));
    }

    let backup_dir = state.storage.get_service_backup_dir(&service.id);
    let file_path = backup_dir.join(&backup.file_name);

    let body = if backup.stored_locally && fs::try_exists(&file_path).await.unwrap_or(false) {
        let file = fs::File::open(&file_path).await?;
        Body::from_stream(ReaderStream::new(file))
    } else if let Some(ref s3_id) = backup.s3_storage_id {
        let storage = S3StorageRepo::find(&state.storage.db_pool, s3_id).await?;
        let key = service_backup_s3_key(&service.id, &backup.file_name);
        let stream = s3::get_object_stream(&storage, &key).await.map_err(|e| {
            HttpError::not_found(format!("Failed to retrieve backup from S3: {}", e))
        })?;
        Body::from_stream(stream)
    } else {
        return Err(HttpError::not_found(format!(
            "Backup file \"{}\" not found on disk or storage",
            backup.file_name
        )));
    };

    let mut builder = Response::builder()
        .header(header::CONTENT_TYPE, "application/octet-stream")
        .header(
            header::CONTENT_DISPOSITION,
            format!("attachment; filename=\"{}\"", backup.file_name),
        );

    if backup.file_size > 0 {
        builder = builder.header(header::CONTENT_LENGTH, backup.file_size);
    }

    let response = builder.body(body).map_err(HttpError::internal)?;

    Ok(response)
}

async fn restore_service_backup(
    State(state): State<AppState>,
    ActiveAppOwner { app, .. }: ActiveAppOwner,
    Path((_, id, backup_id)): Path<(String, String, String)>,
) -> HttpResult<impl IntoResponse> {
    let service_docker = ServiceDocker::new(state, app).await?;
    service_docker.run_service_restore(&id, &backup_id).await?;
    Ok(Json(json!({ "restored": true })))
}

async fn delete_service_backup(
    State(state): State<AppState>,
    ActiveAppOwner { app, .. }: ActiveAppOwner,
    Path((_, id, backup_id)): Path<(String, String, String)>,
) -> HttpResult<impl IntoResponse> {
    let service_docker = ServiceDocker::new(state, app).await?;
    service_docker
        .delete_service_backup(&id, &backup_id)
        .await?;
    Ok(Json(json!({ "deleted": true })))
}
