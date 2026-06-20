use axum::{
    Json, Router,
    extract::{Path, State},
    response::IntoResponse,
    routing::{get, post},
};
use bollard::Docker;
use chrono::Utc;
use serde::{Deserialize, Serialize};
use slasha_db::{
    app_backup::AppBackup,
    repos::{app::AppRepo, app_backup::AppBackupRepo},
};
use uuid::Uuid;

use crate::{
    AppState,
    docker::deployment::{container::is_web_running, litestream},
    error::{HttpError, HttpResult},
    extractors::auth::AuthUser,
    state::Storage,
};

pub fn router() -> Router<AppState> {
    Router::new()
        .route("/", get(get_backup).put(save_backup).delete(delete_backup))
        .route("/restore", post(restore_backup))
        .route("/status", get(backup_status))
        .route("/status/refresh", post(refresh_status))
}

/// API view of a backup config. The secret access key is never returned;
/// callers learn only whether one is set.
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
    AuthUser(user): AuthUser,
    Path(slug): Path<String>,
) -> HttpResult<impl IntoResponse> {
    let app = AppRepo::find_by_slug_for_user(&storage.db_pool, &slug, &user.id).await?;
    let backup = AppBackupRepo::get(&storage.db_pool, &app.id).await?;

    Ok(Json(serde_json::json!({
        "backup": backup.map(BackupView::from),
    })))
}

#[derive(Deserialize)]
struct SaveBackupRequest {
    enabled: bool,
    db_path: String,
    bucket: String,
    endpoint: String,
    path_prefix: Option<String>,
    access_key_id: String,
    /// Optional: omit (or send empty) to keep the existing secret unchanged.
    secret_access_key: Option<String>,
}

async fn save_backup(
    State(storage): State<Storage>,
    AuthUser(user): AuthUser,
    Path(slug): Path<String>,
    Json(payload): Json<SaveBackupRequest>,
) -> HttpResult<impl IntoResponse> {
    let app = AppRepo::find_by_slug_for_user(&storage.db_pool, &slug, &user.id).await?;
    let existing = AppBackupRepo::get(&storage.db_pool, &app.id).await?;

    let secret_access_key = match payload.secret_access_key {
        Some(s) if !s.is_empty() => s,
        _ => existing
            .as_ref()
            .map(|b| b.secret_access_key.clone())
            .unwrap_or_default(),
    };

    // These values are interpolated into the generated litestream config and
    // process environment, so reject control characters that could break either.
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

    let now = Utc::now().naive_utc();
    let backup = AppBackup {
        id: existing
            .as_ref()
            .map(|b| b.id.clone())
            .unwrap_or_else(|| Uuid::new_v4().to_string()),
        app_id: app.id.clone(),
        enabled: payload.enabled,
        db_path: payload.db_path,
        bucket: payload.bucket,
        endpoint: payload.endpoint,
        path_prefix: payload.path_prefix.filter(|p| !p.is_empty()),
        access_key_id: payload.access_key_id,
        secret_access_key,
        restore_pending: existing.as_ref().map(|b| b.restore_pending).unwrap_or(false),
        last_synced_at: existing.as_ref().and_then(|b| b.last_synced_at),
        created_at: existing.as_ref().map(|b| b.created_at).unwrap_or(now),
        updated_at: now,
    };

    let saved = AppBackupRepo::upsert(&storage.db_pool, backup).await?;

    Ok(Json(serde_json::json!({
        "backup": BackupView::from(saved),
    })))
}

async fn delete_backup(
    State(storage): State<Storage>,
    AuthUser(user): AuthUser,
    Path(slug): Path<String>,
) -> HttpResult<impl IntoResponse> {
    let app = AppRepo::find_by_slug_for_user(&storage.db_pool, &slug, &user.id).await?;
    AppBackupRepo::delete(&storage.db_pool, &app.id).await?;

    Ok(Json(serde_json::json!({ "ok": true })))
}

async fn restore_backup(
    State(storage): State<Storage>,
    AuthUser(user): AuthUser,
    Path(slug): Path<String>,
) -> HttpResult<impl IntoResponse> {
    let app = AppRepo::find_by_slug_for_user(&storage.db_pool, &slug, &user.id).await?;
    let backup = AppBackupRepo::get(&storage.db_pool, &app.id).await?;

    let Some(backup) = backup else {
        return Err(HttpError::bad_request("Backups are not configured for this app"));
    };
    if !backup.enabled {
        return Err(HttpError::bad_request("Backups are not enabled for this app"));
    }

    AppBackupRepo::set_restore_pending(&storage.db_pool, &app.id, true).await?;

    Ok(Json(serde_json::json!({ "ok": true })))
}

#[derive(Serialize)]
struct BackupStatus {
    enabled: bool,
    restore_pending: bool,
    /// Whether a web container is currently running (Litestream replicates for
    /// its lifetime).
    web_running: bool,
    last_synced_at: Option<chrono::NaiveDateTime>,
}

async fn backup_status(
    State(storage): State<Storage>,
    State(docker): State<Docker>,
    AuthUser(user): AuthUser,
    Path(slug): Path<String>,
) -> HttpResult<impl IntoResponse> {
    let app = AppRepo::find_by_slug_for_user(&storage.db_pool, &slug, &user.id).await?;
    let backup = AppBackupRepo::get(&storage.db_pool, &app.id).await?;

    let Some(backup) = backup else {
        return Ok(Json(serde_json::json!({
            "status": BackupStatus {
                enabled: false,
                restore_pending: false,
                web_running: false,
                last_synced_at: None,
            },
        })));
    };

    let web_running = is_web_running(&docker, &app.id).await.unwrap_or(false);

    Ok(Json(serde_json::json!({
        "status": BackupStatus {
            enabled: backup.enabled,
            restore_pending: backup.restore_pending,
            web_running,
            last_synced_at: backup.last_synced_at,
        },
    })))
}

/// Query the replica for the latest replicated timestamp and persist it. This
/// runs a one-shot container against object storage, so it's an explicit action
/// rather than part of the cheap polling status.
async fn refresh_status(
    State(storage): State<Storage>,
    State(docker): State<Docker>,
    AuthUser(user): AuthUser,
    Path(slug): Path<String>,
) -> HttpResult<impl IntoResponse> {
    let app = AppRepo::find_by_slug_for_user(&storage.db_pool, &slug, &user.id).await?;
    let backup = AppBackupRepo::get(&storage.db_pool, &app.id).await?;

    let Some(backup) = backup.filter(|b| b.enabled) else {
        return Err(HttpError::bad_request("Backups are not enabled for this app"));
    };

    let last_synced = litestream::probe_last_synced(&docker, &backup)
        .await
        .map_err(|e| HttpError::internal(anyhow::anyhow!("Failed to read replica: {e}")))?;

    if let Some(synced_at) = last_synced {
        AppBackupRepo::set_last_synced(&storage.db_pool, &app.id, synced_at).await?;
    }

    Ok(Json(serde_json::json!({ "last_synced_at": last_synced })))
}
