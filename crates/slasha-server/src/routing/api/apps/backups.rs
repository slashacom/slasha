use axum::{
    Json, Router,
    extract::State,
    response::IntoResponse,
    routing::{get, post},
};
use bollard::Docker;
use chrono::Utc;
use garde::Validate;
use crate::routing::api::validation::not_empty;
use serde::{Deserialize, Serialize};
use slasha_db::{
    app_backup::{AppBackup, NewAppBackup},
    models::app_scale::ProcessType,
    repos::app_backup::AppBackupRepo,
};

use crate::{
    AppState, HttpError, HttpResult,
    docker::deployment::{container::is_web_running, litestream},
    extractors::{ValidatedJson, app::ActiveApp},
    routing::api::deserialize::{trim_optional_string, trim_string},
    state::Storage,
};

pub fn router() -> Router<AppState> {
    Router::new()
        .route("/", get(get_backup).put(save_backup).delete(delete_backup))
        .route("/restore", post(restore_backup))
        .route("/status", get(backup_status))
        .route("/status/refresh", post(refresh_status))
}

#[derive(Serialize)]
struct BackupView {
    enabled: bool,
    db_path: String,
    bucket: String,
    endpoint: String,
    path_prefix: Option<String>,
    access_key_id: String,
    secret_set: bool,
    restore_pending: bool,
    last_synced_at: Option<chrono::NaiveDateTime>,
}

impl From<AppBackup> for BackupView {
    fn from(backup: AppBackup) -> Self {
        let secret_set = !backup.secret_access_key.is_empty();
        BackupView {
            enabled: backup.enabled,
            db_path: backup.db_path,
            bucket: backup.bucket,
            endpoint: backup.endpoint,
            path_prefix: backup.path_prefix,
            access_key_id: backup.access_key_id,
            secret_set,
            restore_pending: backup.restore_pending,
            last_synced_at: backup.last_synced_at,
        }
    }
}

async fn get_backup(
    State(storage): State<Storage>,
    ActiveApp { app, .. }: ActiveApp,
) -> HttpResult<impl IntoResponse> {
    let backup = AppBackupRepo::get(&storage.db_pool, &app.id).await?;

    Ok(Json(serde_json::json!({
        "backup": backup.map(BackupView::from),
    })))
}

#[derive(Deserialize, Validate)]
struct SaveBackupRequest {
    #[garde(skip)]
    enabled: bool,
    #[serde(deserialize_with = "trim_string")]
    #[garde(custom(not_empty))]
    db_path: String,
    #[serde(deserialize_with = "trim_string")]
    #[garde(skip)]
    bucket: String,
    #[serde(deserialize_with = "trim_string")]
    #[garde(custom(not_empty))]
    endpoint: String,
    #[serde(default, deserialize_with = "trim_optional_string")]
    #[garde(skip)]
    path_prefix: Option<String>,
    #[serde(deserialize_with = "trim_string")]
    #[garde(skip)]
    access_key_id: String,
    #[serde(default, deserialize_with = "trim_optional_string")]
    #[garde(skip)]
    secret_access_key: Option<String>,
}

async fn save_backup(
    State(storage): State<Storage>,
    ActiveApp { app, .. }: ActiveApp,
    ValidatedJson(payload): ValidatedJson<SaveBackupRequest>,
) -> HttpResult<impl IntoResponse> {
    let existing = AppBackupRepo::get(&storage.db_pool, &app.id).await?;

    // keep current secret if not provided
    let secret_access_key = match payload.secret_access_key {
        Some(s) if !s.is_empty() => s,
        _ => existing
            .as_ref()
            .map(|b| b.secret_access_key.clone())
            .unwrap_or_default(),
    };

    let fields = [
        Some(&payload.db_path),
        Some(&payload.bucket),
        Some(&payload.endpoint),
        Some(&payload.access_key_id),
        Some(&secret_access_key),
        payload.path_prefix.as_ref(),
    ];
    if fields
        .into_iter()
        .flatten()
        .any(|v| v.chars().any(|c| c.is_control()))
    {
        return Err(HttpError::bad_request(
            "Backup settings must not contain control characters",
        ));
    }

    if payload.enabled && (payload.bucket.is_empty() || secret_access_key.is_empty()) {
        return Err(HttpError::bad_request(
            "Bucket and credentials are required to enable backups",
        ));
    }

    // litestream only allows one writer; multiple web instances would cause db corruption
    if payload.enabled {
        let scale_configs =
            slasha_db::repos::app_scale::AppScaleRepo::list_for_app(&storage.db_pool, &app.id)
                .await?;
        if scale_configs
            .iter()
            .any(|s| s.process_type == ProcessType::Web && s.desired > 1)
        {
            return Err(HttpError::bad_request(
                "Cannot enable backups while the web process is scaled beyond 1 instance. Scale down to 1 first.",
            ));
        }
    }

    let backup = NewAppBackup {
        app_id: app.id.clone(),
        enabled: payload.enabled,
        db_path: payload.db_path,
        bucket: payload.bucket,
        endpoint: payload.endpoint,
        path_prefix: payload.path_prefix.filter(|p| !p.is_empty()),
        access_key_id: payload.access_key_id,
        secret_access_key,
    };

    let saved = AppBackupRepo::upsert(&storage.db_pool, backup).await?;

    Ok(Json(serde_json::json!({
        "backup": BackupView::from(saved),
    })))
}

async fn delete_backup(
    State(storage): State<Storage>,
    ActiveApp { app, .. }: ActiveApp,
) -> HttpResult<impl IntoResponse> {
    AppBackupRepo::delete(&storage.db_pool, &app.id).await?;

    Ok(Json(serde_json::json!({ "ok": true })))
}

async fn restore_backup(
    State(storage): State<Storage>,
    ActiveApp { app, .. }: ActiveApp,
) -> HttpResult<impl IntoResponse> {
    let backup = AppBackupRepo::get(&storage.db_pool, &app.id).await?;

    let Some(backup) = backup else {
        return Err(HttpError::bad_request(
            "Backups are not configured for this app",
        ));
    };
    if !backup.enabled {
        return Err(HttpError::bad_request(
            "Backups are not enabled for this app",
        ));
    }

    AppBackupRepo::set_restore_pending(&storage.db_pool, &app.id, true).await?;

    Ok(Json(serde_json::json!({ "ok": true })))
}

#[derive(Serialize)]
struct BackupStatus {
    enabled: bool,
    restore_pending: bool,
    web_running: bool,
    last_synced_at: Option<chrono::NaiveDateTime>,
    last_checked_at: Option<chrono::NaiveDateTime>,
    healthy: Option<bool>,
    health_error: Option<String>,
}

impl BackupStatus {
    fn disabled() -> Self {
        BackupStatus {
            enabled: false,
            restore_pending: false,
            web_running: false,
            last_synced_at: None,
            last_checked_at: None,
            healthy: None,
            health_error: None,
        }
    }
}

async fn backup_status(
    State(storage): State<Storage>,
    State(docker): State<Docker>,
    ActiveApp { app, .. }: ActiveApp,
) -> HttpResult<impl IntoResponse> {
    let backup = AppBackupRepo::get(&storage.db_pool, &app.id).await?;

    let Some(backup) = backup else {
        return Ok(Json(
            serde_json::json!({ "status": BackupStatus::disabled() }),
        ));
    };

    let web_running = is_web_running(&docker, &app.id).await.unwrap_or(false);

    Ok(Json(serde_json::json!({
        "status": BackupStatus {
            enabled: backup.enabled,
            restore_pending: backup.restore_pending,
            web_running,
            last_synced_at: backup.last_synced_at,
            last_checked_at: backup.last_checked_at,
            healthy: backup.last_check_ok,
            health_error: backup.last_check_error,
        },
    })))
}

async fn refresh_status(
    State(storage): State<Storage>,
    State(docker): State<Docker>,
    ActiveApp { app, .. }: ActiveApp,
) -> HttpResult<impl IntoResponse> {
    let backup = AppBackupRepo::get(&storage.db_pool, &app.id).await?;

    let Some(backup) = backup.filter(|b| b.enabled) else {
        return Err(HttpError::bad_request(
            "Backups are not enabled for this app",
        ));
    };

    let probe = litestream::probe_replica(&docker, &backup)
        .await
        .map_err(|e| HttpError::internal(anyhow::anyhow!("Failed to probe replica: {e}")))?;

    AppBackupRepo::set_health(
        &storage.db_pool,
        &app.id,
        Utc::now().naive_utc(),
        probe.reachable,
        probe.error.clone(),
        probe.last_synced_at,
    )
    .await?;

    Ok(Json(serde_json::json!({
        "healthy": probe.reachable,
        "health_error": probe.error,
        "last_synced_at": probe.last_synced_at,
    })))
}
