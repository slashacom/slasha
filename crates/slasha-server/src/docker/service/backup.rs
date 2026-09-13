use std::{collections::HashMap, path::Path};

use bollard::Docker;
use bytes::Bytes;
use futures_util::stream::BoxStream;
use slasha_db::{
    logs::ResourceKind,
    models::service::{Service, ServiceKind},
    repos::{s3_storage::S3StorageRepo, service::ServiceRepo, service_backup::ServiceBackupRepo},
    service::ServiceStatus,
    service_backup::{
        ServiceBackup, ServiceBackupConfig, ServiceBackupStatus, ServiceRestoreStatus,
    },
};
use tokio::{fs, io};
use tokio_util::io::ReaderStream;
use tracing::{error, warn};

use crate::{
    docker::{
        DockerError, DockerResult,
        exec::start_container_exec,
        naming::service_container_name,
        service::{
            ServiceDocker, env::resolve_service_env, provision::instance::wait_for_service_health,
            spec::ServiceKindDockerExt,
        },
        utils::{restart_container, stop_container},
    },
    s3,
    state::AppState,
};

/// Generates the S3 storage object key for a database service backup.
///
/// # Arguments
///
/// * `service_id` - Target database service ID string.
/// * `file_name` - Backup snapshot file name.
///
/// # Returns
///
/// Formatted S3 object key string.
pub fn service_backup_s3_key(service_id: &str, file_name: &str) -> String {
    format!("services/{}/{}", service_id, file_name)
}

/// Executes a database service backup asynchronously, streaming the dump to disk/S3, pruning retention, and updating DB status.
///
/// # Arguments
///
/// * `service_docker` - Service Docker orchestration instance ([`ServiceDocker`]).
/// * `service` - Target database service model ([`Service`]).
/// * `backup` - Active backup record in running state ([`ServiceBackup`]).
/// * `resolved_env` - Map of resolved environment key-value pairs.
/// * `config` - Optional backup configuration for retention pruning ([`ServiceBackupConfig`]).
pub async fn execute_service_backup(
    service_docker: ServiceDocker,
    service: Service,
    backup: ServiceBackup,
    resolved_env: HashMap<String, String>,
    config: Option<ServiceBackupConfig>,
) {
    let keep_local = config.as_ref().map(|c| c.keep_local).unwrap_or(true);
    let db_pool = service_docker.state.storage.db_pool.clone();
    let backup_dir = service_docker
        .state
        .storage
        .get_service_backup_dir(&service.id);
    let file_path = backup_dir.join(&backup.file_name);

    match process_service_backup(
        &service_docker,
        &service,
        &backup,
        &resolved_env,
        keep_local,
        &file_path,
    )
    .await
    {
        Ok(outcome) => {
            if let Err(e) = ServiceBackupRepo::mark_backup_finished(
                &db_pool,
                &backup.id,
                ServiceBackupStatus::Succeeded,
                outcome.file_size,
                outcome.stored_locally,
                outcome.s3_error,
            )
            .await
            {
                error!(
                    target: "slasha::service_backup",
                    backup_id = %backup.id,
                    error = ?e,
                    "failed marking backup as succeeded"
                );
            }

            if let Some(cfg) = config
                && cfg.retention_count > 0
            {
                prune_excess_backups(
                    &service_docker.state,
                    &service,
                    ServiceBackupStatus::Succeeded,
                    cfg.retention_count,
                )
                .await;
            }
        }
        Err(err) => {
            remove_local_file(&file_path).await;

            if let Some(ref s3_id) = backup.s3_storage_id
                && let Ok(storage) =
                    S3StorageRepo::find(&service_docker.state.storage.db_pool, s3_id).await
            {
                let key = service_backup_s3_key(&service.id, &backup.file_name);
                let _ = s3::delete_file(&storage, &key).await;
            }

            error!(
                target: "slasha::service_backup",
                service_id = %service.id,
                backup_id = %backup.id,
                error = %err,
                "service backup failed"
            );

            let _ = ServiceBackupRepo::mark_backup_finished(
                &db_pool,
                &backup.id,
                ServiceBackupStatus::Failed,
                0,
                false,
                Some(err),
            )
            .await;
        }
    }
}

/// Executes a database service restoration asynchronously from local disk or S3 and updates restore status.
///
/// # Arguments
///
/// * `service_docker` - Service Docker orchestration instance ([`ServiceDocker`]).
/// * `service` - Target database service model ([`Service`]).
/// * `backup` - Backup snapshot record to restore ([`ServiceBackup`]).
pub async fn execute_service_restore(
    service_docker: ServiceDocker,
    service: Service,
    backup: ServiceBackup,
) {
    let container_name = service_container_name(&service.id);
    let db_pool = service_docker.state.storage.db_pool.clone();
    let _ = ServiceBackupRepo::set_restore_status(
        &db_pool,
        &backup.id,
        ServiceRestoreStatus::Restoring,
        None,
    )
    .await;

    let res = async {
        restore_service_from_backup(&service_docker, &service, &backup).await?;

        restart_container(&service_docker.docker_client, &container_name).await?;

        let log_writer = service_docker
            .state
            .runtime
            .log_bus
            .writer(ResourceKind::Service, &service.id)
            .app_id(&service_docker.app.id);

        wait_for_service_health(
            &service_docker.docker_client,
            &container_name,
            &service.name,
            180,
            &log_writer,
        )
        .await?;

        Ok::<(), DockerError>(())
    }
    .await;

    match res {
        Ok(()) => {
            let _ = ServiceBackupRepo::set_restore_status(
                &db_pool,
                &backup.id,
                ServiceRestoreStatus::Succeeded,
                None,
            )
            .await;
        }
        Err(e) => {
            error!(
                target: "slasha::service_backup",
                service_id = %service.id,
                backup_id = %backup.id,
                error = ?e,
                "service restore failed"
            );
            let _ = ServiceBackupRepo::set_restore_status(
                &db_pool,
                &backup.id,
                ServiceRestoreStatus::Failed,
                Some(e.to_string()),
            )
            .await;
            let _ = ServiceRepo::update_status(&db_pool, &service.id, ServiceStatus::Failed).await;
            let _ = stop_container(&service_docker.docker_client, &container_name, Some(10)).await;
        }
    }
}

/// Deletes backup snapshot files from disk and optional S3 storage.
///
/// # Arguments
///
/// * `state` - Application state holding storage configuration ([`AppState`]).
/// * `service` - Target database service model ([`Service`]).
/// * `backup` - Backup snapshot record ([`ServiceBackup`]).
///
/// # Returns
///
/// A [`DockerResult`] indicating file deletion success.
pub async fn delete_service_backup_files(
    state: &AppState,
    service: &Service,
    backup: &ServiceBackup,
) -> DockerResult<()> {
    if backup.stored_locally {
        let backup_dir = state.storage.get_service_backup_dir(&service.id);
        let file_path = backup_dir.join(&backup.file_name);
        remove_local_file(&file_path).await;
    }

    if let Some(ref s3_id) = backup.s3_storage_id
        && let Ok(storage) = S3StorageRepo::find(&state.storage.db_pool, s3_id).await
    {
        let key = service_backup_s3_key(&service.id, &backup.file_name);
        if let Err(e) = s3::delete_file(&storage, &key).await {
            warn!(
                target: "slasha::service_backup",
                key = %key,
                error = ?e,
                "failed deleting S3 backup object"
            );
        }
    }

    Ok(())
}

struct BackupResult {
    file_size: i64,
    stored_locally: bool,
    s3_error: Option<String>,
}

/// Executes the database dump and handles optional S3 upload and local retention.
///
/// # Arguments
///
/// * `service_docker` - Service Docker orchestration instance ([`ServiceDocker`]).
/// * `service` - Target database service model ([`Service`]).
/// * `backup` - Active backup record ([`ServiceBackup`]).
/// * `resolved_env` - Map of resolved environment key-value pairs.
/// * `keep_local` - Whether to retain the snapshot file on local disk (`bool`).
/// * `file_path` - Local snapshot destination path ([`Path`]).
///
/// # Returns
///
/// A [`Result`] containing [`BackupResult`] on success, or an error description string on failure.
async fn process_service_backup(
    service_docker: &ServiceDocker,
    service: &Service,
    backup: &ServiceBackup,
    resolved_env: &HashMap<String, String>,
    keep_local: bool,
    file_path: &Path,
) -> Result<BackupResult, String> {
    let file_size = backup_service_to_file(
        &service_docker.docker_client,
        service,
        resolved_env,
        file_path,
    )
    .await
    .map_err(|e| e.to_string())?;

    let mut s3_error = None;
    let mut s3_succeeded = false;

    if let Some(ref s3_id) = backup.s3_storage_id {
        match S3StorageRepo::find(&service_docker.state.storage.db_pool, s3_id).await {
            Ok(storage) => {
                let key = service_backup_s3_key(&service.id, &backup.file_name);
                if let Err(e) = s3::upload_file(&storage, &key, file_path).await {
                    warn!(
                        target: "slasha::service_backup",
                        service_id = %service.id,
                        backup_id = %backup.id,
                        s3_id = %s3_id,
                        error = ?e,
                        "failed uploading backup to S3 storage"
                    );
                    s3_error = Some(format!("S3 upload failed: {}", e));
                } else {
                    s3_succeeded = true;
                }
            }
            Err(e) => {
                warn!(
                    target: "slasha::service_backup",
                    service_id = %service.id,
                    backup_id = %backup.id,
                    s3_id = %s3_id,
                    error = ?e,
                    "S3 storage destination not found"
                );
                s3_error = Some(format!("S3 storage not found: {}", e));
            }
        }
    }

    if s3_error.is_some() && !keep_local {
        remove_local_file(file_path).await;
        return Err(s3_error.unwrap_or_else(|| "S3 upload failed".into()));
    }

    let stored_locally = if !keep_local && s3_succeeded {
        remove_local_file(file_path).await;
        false
    } else {
        true
    };

    Ok(BackupResult {
        file_size,
        stored_locally,
        s3_error,
    })
}

/// Dumps a database service backup from its container directly to a destination file on disk.
///
/// # Arguments
///
/// * `docker_client` - Docker API client ([`Docker`]).
/// * `service` - Target database service model ([`Service`]).
/// * `resolved_env` - Map of resolved environment key-value pairs.
/// * `file_path` - Local filesystem destination path ([`Path`]).
///
/// # Returns
///
/// A [`DockerResult`] containing the written backup file size in bytes (`i64`).
async fn backup_service_to_file(
    docker_client: &Docker,
    service: &Service,
    resolved_env: &HashMap<String, String>,
    file_path: &Path,
) -> DockerResult<i64> {
    let (cmd, env) = service.kind.backup_exec(resolved_env);
    let container_name = service_container_name(&service.id);
    let mut exec = start_container_exec(docker_client, &container_name, cmd, env, false).await?;

    let (file_size, stderr) = exec.stream_stdout_to_file(file_path).await?;

    match exec.inspect_exit_code().await? {
        Some(0) => {}
        Some(code) => return Err(DockerError::ServiceBackupFailed(code, stderr)),
        None => {
            return Err(DockerError::ServiceBackupFailed(
                1,
                "backup exec exited with unknown status".into(),
            ));
        }
    }

    if file_size == 0 {
        return Err(DockerError::ServiceBackupFailed(
            1,
            format!("backup snapshot is empty (0 bytes): {}", stderr),
        ));
    }

    Ok(file_size)
}

/// Restores a database service from a local snapshot file or remote S3 object stream.
///
/// # Arguments
///
/// * `service_docker` - Service Docker orchestration instance ([`ServiceDocker`]).
/// * `service` - Target database service model ([`Service`]).
/// * `backup` - Backup snapshot record to restore ([`ServiceBackup`]).
///
/// # Returns
///
/// A [`DockerResult`] indicating restore completion success.
async fn restore_service_from_backup(
    service_docker: &ServiceDocker,
    service: &Service,
    backup: &ServiceBackup,
) -> DockerResult<()> {
    let backup_dir = service_docker
        .state
        .storage
        .get_service_backup_dir(&service.id);
    let file_path = backup_dir.join(&backup.file_name);

    let s3_source = if backup.stored_locally && file_path.exists() {
        None
    } else if let Some(ref s3_id) = backup.s3_storage_id {
        let storage = S3StorageRepo::find(&service_docker.state.storage.db_pool, s3_id).await?;
        let key = service_backup_s3_key(&service.id, &backup.file_name);
        Some((storage, key))
    } else {
        return Err(DockerError::ArtifactUnavailable(format!(
            "backup file '{}' not found on local disk and no S3 storage attached",
            backup.file_name
        )));
    };

    let env_vars =
        ServiceRepo::get_env_vars(&service_docker.state.storage.db_pool, &service.id).await?;
    let resolved = resolve_service_env(env_vars, service)?;
    let container_name = service_container_name(&service.id);

    let (cmd, env) = service.kind.restore_exec(&resolved);
    let mut exec = start_container_exec(
        &service_docker.docker_client,
        &container_name,
        cmd,
        env,
        true,
    )
    .await?;

    let stream: BoxStream<'static, Result<Bytes, io::Error>> = match s3_source {
        Some((storage, key)) => s3::get_object_stream(&storage, &key)
            .await
            .map_err(io::Error::other)?,
        None => {
            let file = fs::File::open(&file_path).await?;
            Box::pin(ReaderStream::new(file))
        }
    };

    let stderr_buf = exec.pipe_stream_and_drain_stderr(stream).await?;

    match exec.inspect_exit_code().await? {
        Some(0) => {}
        Some(1) if service.kind == ServiceKind::PostgreSQL => {
            warn!(
                target: "slasha::service_backup",
                service_id = %service.id,
                stderr = %stderr_buf,
                "pg_restore completed with non-fatal warnings (exit code 1)"
            );
        }
        Some(code) => {
            return Err(DockerError::ServiceRestoreFailed(code, stderr_buf));
        }
        None => {
            return Err(DockerError::ServiceRestoreFailed(
                1,
                "restore exec exited with unknown status".into(),
            ));
        }
    }

    Ok(())
}

/// Prunes excess service backups exceeding the retention threshold from storage and database.
///
/// # Arguments
///
/// * `state` - Application state holding database and storage configuration ([`AppState`]).
/// * `service` - Target database service model ([`Service`]).
/// * `status` - Status filter for candidate backups ([`ServiceBackupStatus`]).
/// * `retention_count` - Maximum number of recent snapshots to retain (`i32`).
async fn prune_excess_backups(
    state: &AppState,
    service: &Service,
    status: ServiceBackupStatus,
    retention_count: i32,
) {
    let Ok(excess) = ServiceBackupRepo::list_excess(
        &state.storage.db_pool,
        &service.id,
        status,
        retention_count,
    )
    .await
    else {
        return;
    };

    for old in excess {
        let _ = delete_service_backup_files(state, service, &old).await;
        if let Err(e) = ServiceBackupRepo::delete_backup(&state.storage.db_pool, &old.id).await {
            warn!(
                target: "slasha::service_backup",
                backup_id = %old.id,
                error = ?e,
                "failed deleting excess backup record from database"
            );
        }
    }
}

/// Deletes a local file if it exists, logging a warning on unexpected I/O errors.
///
/// # Arguments
///
/// * `file_path` - Local file path to remove ([`Path`]).
async fn remove_local_file(file_path: &Path) {
    if let Err(e) = fs::remove_file(file_path).await
        && e.kind() != io::ErrorKind::NotFound
    {
        warn!(
            target: "slasha::service_backup",
            file = %file_path.display(),
            error = ?e,
            "failed removing local backup file"
        );
    }
}
