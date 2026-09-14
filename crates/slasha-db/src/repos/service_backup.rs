use chrono::{NaiveDateTime, Utc};
use diesel::{prelude::*, upsert::excluded};

use crate::{
    connection::DbPool,
    error::{DbError, DbResult},
    models::{
        schema::{service_backup_configs, service_backups},
        service_backup::{
            NewServiceBackup, NewServiceBackupConfig, ServiceBackup, ServiceBackupConfig,
            ServiceBackupStatus, ServiceBackupTrigger, ServiceRestoreStatus,
        },
    },
};

pub struct ServiceBackupRepo;

impl ServiceBackupRepo {
    pub async fn get_config(
        pool: &DbPool,
        service_id: &str,
    ) -> DbResult<Option<ServiceBackupConfig>> {
        let pool = pool.clone();
        let service_id = service_id.to_string();
        tokio::task::spawn_blocking(move || {
            let mut conn = pool.get()?;
            Ok(service_backup_configs::table
                .filter(service_backup_configs::service_id.eq(&service_id))
                .first::<ServiceBackupConfig>(&mut conn)
                .optional()?)
        })
        .await?
    }

    pub async fn upsert_config(
        pool: &DbPool,
        config: NewServiceBackupConfig,
    ) -> DbResult<ServiceBackupConfig> {
        let pool = pool.clone();
        tokio::task::spawn_blocking(move || {
            let mut conn = pool.get()?;
            let now = Utc::now().naive_utc();

            let upserted: ServiceBackupConfig = diesel::insert_into(service_backup_configs::table)
                .values((
                    service_backup_configs::service_id.eq(&config.service_id),
                    service_backup_configs::enabled.eq(config.enabled),
                    service_backup_configs::schedule.eq(&config.schedule),
                    service_backup_configs::timezone.eq(&config.timezone),
                    service_backup_configs::retention_count.eq(config.retention_count),
                    service_backup_configs::s3_storage_id.eq(&config.s3_storage_id),
                    service_backup_configs::keep_local.eq(config.keep_local),
                    service_backup_configs::next_run_at.eq(config.next_run_at),
                    service_backup_configs::updated_at.eq(now),
                ))
                .on_conflict(service_backup_configs::service_id)
                .do_update()
                .set((
                    service_backup_configs::enabled.eq(excluded(service_backup_configs::enabled)),
                    service_backup_configs::schedule.eq(excluded(service_backup_configs::schedule)),
                    service_backup_configs::timezone.eq(excluded(service_backup_configs::timezone)),
                    service_backup_configs::retention_count
                        .eq(excluded(service_backup_configs::retention_count)),
                    service_backup_configs::s3_storage_id
                        .eq(excluded(service_backup_configs::s3_storage_id)),
                    service_backup_configs::keep_local
                        .eq(excluded(service_backup_configs::keep_local)),
                    service_backup_configs::next_run_at
                        .eq(excluded(service_backup_configs::next_run_at)),
                    service_backup_configs::updated_at.eq(now),
                ))
                .returning(ServiceBackupConfig::as_returning())
                .get_result(&mut conn)?;

            Ok(upserted)
        })
        .await?
    }

    pub async fn list_due_configs(
        pool: &DbPool,
        now: NaiveDateTime,
    ) -> DbResult<Vec<ServiceBackupConfig>> {
        let pool = pool.clone();
        tokio::task::spawn_blocking(move || {
            let mut conn = pool.get()?;
            Ok(service_backup_configs::table
                .filter(service_backup_configs::enabled.eq(true))
                .filter(
                    service_backup_configs::next_run_at
                        .is_null()
                        .or(service_backup_configs::next_run_at.le(now)),
                )
                .load::<ServiceBackupConfig>(&mut conn)?)
        })
        .await?
    }

    pub async fn update_schedule_state(
        pool: &DbPool,
        service_id: &str,
        last_run_at: Option<NaiveDateTime>,
        next_run_at: Option<NaiveDateTime>,
    ) -> DbResult<()> {
        let pool = pool.clone();
        let service_id = service_id.to_string();
        tokio::task::spawn_blocking(move || {
            let mut conn = pool.get()?;
            diesel::update(
                service_backup_configs::table
                    .filter(service_backup_configs::service_id.eq(&service_id)),
            )
            .set((
                service_backup_configs::last_run_at.eq(last_run_at),
                service_backup_configs::next_run_at.eq(next_run_at),
                service_backup_configs::updated_at.eq(Utc::now().naive_utc()),
            ))
            .execute(&mut conn)?;
            Ok(())
        })
        .await?
    }

    pub async fn list_backups(
        pool: &DbPool,
        service_id: &str,
        limit: i64,
    ) -> DbResult<Vec<ServiceBackup>> {
        let pool = pool.clone();
        let service_id = service_id.to_string();
        tokio::task::spawn_blocking(move || {
            let mut conn = pool.get()?;
            Ok(service_backups::table
                .filter(service_backups::service_id.eq(&service_id))
                .order(service_backups::created_at.desc())
                .limit(limit)
                .load::<ServiceBackup>(&mut conn)?)
        })
        .await?
    }

    pub async fn find_backup(pool: &DbPool, id: &str) -> DbResult<ServiceBackup> {
        let pool = pool.clone();
        let id = id.to_string();
        tokio::task::spawn_blocking(move || {
            let mut conn = pool.get()?;
            service_backups::table
                .filter(service_backups::id.eq(&id))
                .first::<ServiceBackup>(&mut conn)
                .optional()?
                .ok_or_else(|| DbError::NotFound(format!("service backup '{}' not found", id)))
        })
        .await?
    }

    pub async fn create_backup(pool: &DbPool, backup: NewServiceBackup) -> DbResult<ServiceBackup> {
        let pool = pool.clone();
        tokio::task::spawn_blocking(move || {
            let mut conn = pool.get()?;
            Ok(diesel::insert_into(service_backups::table)
                .values((
                    service_backups::id.eq(&backup.id),
                    service_backups::service_id.eq(&backup.service_id),
                    service_backups::s3_storage_id.eq(&backup.s3_storage_id),
                    service_backups::file_name.eq(&backup.file_name),
                    service_backups::file_size.eq(backup.file_size),
                    service_backups::status.eq(backup.status.to_string()),
                    service_backups::trigger_kind.eq(backup.trigger_kind.to_string()),
                    service_backups::stored_locally.eq(backup.stored_locally),
                    service_backups::restore_status.eq(ServiceRestoreStatus::Idle.to_string()),
                ))
                .returning(ServiceBackup::as_returning())
                .get_result(&mut conn)?)
        })
        .await?
    }

    pub async fn mark_backup_finished(
        pool: &DbPool,
        id: &str,
        status: ServiceBackupStatus,
        file_size: i64,
        stored_locally: bool,
        error: Option<String>,
    ) -> DbResult<()> {
        let pool = pool.clone();
        let id = id.to_string();
        tokio::task::spawn_blocking(move || {
            let mut conn = pool.get()?;
            diesel::update(service_backups::table.filter(service_backups::id.eq(&id)))
                .set((
                    service_backups::status.eq(status.to_string()),
                    service_backups::file_size.eq(file_size),
                    service_backups::stored_locally.eq(stored_locally),
                    service_backups::error.eq(error),
                ))
                .execute(&mut conn)?;
            Ok(())
        })
        .await?
    }

    pub async fn delete_backup(pool: &DbPool, id: &str) -> DbResult<()> {
        let pool = pool.clone();
        let id = id.to_string();
        tokio::task::spawn_blocking(move || {
            let mut conn = pool.get()?;
            diesel::delete(service_backups::table.filter(service_backups::id.eq(&id)))
                .execute(&mut conn)?;
            Ok(())
        })
        .await?
    }

    pub async fn list_excess(
        pool: &DbPool,
        service_id: &str,
        status: ServiceBackupStatus,
        retention_count: i32,
    ) -> DbResult<Vec<ServiceBackup>> {
        let pool = pool.clone();
        let service_id = service_id.to_string();
        tokio::task::spawn_blocking(move || {
            let mut conn = pool.get()?;
            Ok(service_backups::table
                .filter(service_backups::service_id.eq(&service_id))
                .filter(service_backups::status.eq(status.to_string()))
                .filter(
                    service_backups::trigger_kind.eq(ServiceBackupTrigger::Scheduled.to_string()),
                )
                .filter(
                    service_backups::restore_status.ne(ServiceRestoreStatus::Restoring.to_string()),
                )
                .order(service_backups::created_at.desc())
                .offset(retention_count as i64)
                .load::<ServiceBackup>(&mut conn)?)
        })
        .await?
    }

    pub async fn list_for_service_with_s3(
        pool: &DbPool,
        service_id: &str,
    ) -> DbResult<Vec<ServiceBackup>> {
        let pool = pool.clone();
        let service_id = service_id.to_string();
        tokio::task::spawn_blocking(move || {
            let mut conn = pool.get()?;
            Ok(service_backups::table
                .filter(service_backups::service_id.eq(&service_id))
                .filter(service_backups::s3_storage_id.is_not_null())
                .load::<ServiceBackup>(&mut conn)?)
        })
        .await?
    }

    pub async fn count_by_s3_storage(pool: &DbPool, s3_storage_id: &str) -> DbResult<i64> {
        let pool = pool.clone();
        let s3_storage_id = s3_storage_id.to_string();
        tokio::task::spawn_blocking(move || {
            let mut conn = pool.get()?;
            let configs_count: i64 = service_backup_configs::table
                .filter(service_backup_configs::s3_storage_id.eq(&s3_storage_id))
                .count()
                .get_result(&mut conn)?;

            let backups_count: i64 = service_backups::table
                .filter(service_backups::s3_storage_id.eq(&s3_storage_id))
                .filter(service_backups::stored_locally.eq(false))
                .filter(service_backups::status.eq(ServiceBackupStatus::Succeeded.to_string()))
                .count()
                .get_result(&mut conn)?;

            Ok(configs_count + backups_count)
        })
        .await?
    }

    pub async fn set_restore_status(
        pool: &DbPool,
        id: &str,
        status: ServiceRestoreStatus,
        error: Option<String>,
    ) -> DbResult<()> {
        let pool = pool.clone();
        let id = id.to_string();
        tokio::task::spawn_blocking(move || {
            let mut conn = pool.get()?;
            if status == ServiceRestoreStatus::Succeeded {
                diesel::update(service_backups::table.filter(service_backups::id.eq(&id)))
                    .set((
                        service_backups::restore_status.eq(status.to_string()),
                        service_backups::last_restored_at.eq(Some(Utc::now().naive_utc())),
                        service_backups::restore_error.eq(error),
                    ))
                    .execute(&mut conn)?;
            } else {
                diesel::update(service_backups::table.filter(service_backups::id.eq(&id)))
                    .set((
                        service_backups::restore_status.eq(status.to_string()),
                        service_backups::restore_error.eq(error),
                    ))
                    .execute(&mut conn)?;
            }
            Ok(())
        })
        .await?
    }

    pub async fn mark_zombie_backups_failed(pool: &DbPool) -> DbResult<usize> {
        let pool = pool.clone();
        tokio::task::spawn_blocking(move || {
            let mut conn = pool.get()?;
            let count = diesel::update(
                service_backups::table
                    .filter(service_backups::status.eq(ServiceBackupStatus::Running.to_string())),
            )
            .set((
                service_backups::status.eq(ServiceBackupStatus::Failed.to_string()),
                service_backups::error.eq(Some(
                    "Server was restarted while backup was running".to_string(),
                )),
            ))
            .execute(&mut conn)?;

            diesel::update(service_backups::table.filter(
                service_backups::restore_status.eq(ServiceRestoreStatus::Restoring.to_string()),
            ))
            .set((
                service_backups::restore_status.eq(ServiceRestoreStatus::Failed.to_string()),
                service_backups::restore_error.eq(Some(
                    "Server was restarted while restore was in progress".to_string(),
                )),
            ))
            .execute(&mut conn)?;

            Ok(count)
        })
        .await?
    }
}
