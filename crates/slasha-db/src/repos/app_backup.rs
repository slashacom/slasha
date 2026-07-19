use diesel::{prelude::*, upsert::excluded};

use crate::{
    connection::DbPool,
    error::DbResult,
    models::{
        app_backup::{AppBackup, NewAppBackup},
        schema::app_backups,
    },
};

pub struct AppBackupRepo;

impl AppBackupRepo {
    pub async fn get(pool: &DbPool, app_id: &str) -> DbResult<Option<AppBackup>> {
        let pool = pool.clone();
        let app_id = app_id.to_string();
        tokio::task::spawn_blocking(move || {
            let mut conn = pool.get()?;
            Ok(app_backups::table
                .filter(app_backups::app_id.eq(&app_id))
                .first::<AppBackup>(&mut conn)
                .optional()?)
        })
        .await?
    }

    pub async fn upsert(pool: &DbPool, backup: NewAppBackup) -> DbResult<AppBackup> {
        let pool = pool.clone();
        tokio::task::spawn_blocking(move || {
            let mut conn = pool.get()?;
            let id = uuid::Uuid::new_v4().to_string();
            let upserted_backup: AppBackup = diesel::insert_into(app_backups::table)
                .values((
                    app_backups::id.eq(&id),
                    app_backups::app_id.eq(&backup.app_id),
                    app_backups::enabled.eq(backup.enabled),
                    app_backups::db_path.eq(&backup.db_path),
                    app_backups::bucket.eq(&backup.bucket),
                    app_backups::endpoint.eq(&backup.endpoint),
                    app_backups::path_prefix.eq(&backup.path_prefix),
                    app_backups::access_key_id.eq(&backup.access_key_id),
                    app_backups::secret_access_key.eq(&backup.secret_access_key),
                ))
                .on_conflict(app_backups::app_id)
                .do_update()
                .set((
                    app_backups::enabled.eq(excluded(app_backups::enabled)),
                    app_backups::db_path.eq(excluded(app_backups::db_path)),
                    app_backups::bucket.eq(excluded(app_backups::bucket)),
                    app_backups::endpoint.eq(excluded(app_backups::endpoint)),
                    app_backups::path_prefix.eq(excluded(app_backups::path_prefix)),
                    app_backups::access_key_id.eq(excluded(app_backups::access_key_id)),
                    app_backups::secret_access_key.eq(excluded(app_backups::secret_access_key)),
                    app_backups::updated_at.eq(chrono::Utc::now().naive_utc()),
                ))
                .returning(AppBackup::as_returning())
                .get_result(&mut conn)?;

            Ok(upserted_backup)
        })
        .await?
    }

    pub async fn set_restore_pending(pool: &DbPool, app_id: &str, pending: bool) -> DbResult<()> {
        let pool = pool.clone();
        let app_id = app_id.to_string();
        tokio::task::spawn_blocking(move || {
            let mut conn = pool.get()?;
            diesel::update(app_backups::table.filter(app_backups::app_id.eq(&app_id)))
                .set((
                    app_backups::restore_pending.eq(pending),
                    app_backups::updated_at.eq(chrono::Utc::now().naive_utc()),
                ))
                .execute(&mut conn)?;
            Ok(())
        })
        .await?
    }

    pub async fn set_health(
        pool: &DbPool,
        app_id: &str,
        checked_at: chrono::NaiveDateTime,
        ok: bool,
        error: Option<String>,
        last_synced_at: Option<chrono::NaiveDateTime>,
    ) -> DbResult<()> {
        let pool = pool.clone();
        let app_id = app_id.to_string();
        tokio::task::spawn_blocking(move || {
            let mut conn = pool.get()?;
            let query = diesel::update(app_backups::table.filter(app_backups::app_id.eq(&app_id)));
            match last_synced_at {
                Some(synced) => query
                    .set((
                        app_backups::last_checked_at.eq(checked_at),
                        app_backups::last_check_ok.eq(ok),
                        app_backups::last_check_error.eq(error),
                        app_backups::last_synced_at.eq(synced),
                    ))
                    .execute(&mut conn)?,
                None => query
                    .set((
                        app_backups::last_checked_at.eq(checked_at),
                        app_backups::last_check_ok.eq(ok),
                        app_backups::last_check_error.eq(error),
                    ))
                    .execute(&mut conn)?,
            };
            Ok(())
        })
        .await?
    }

    pub async fn delete(pool: &DbPool, app_id: &str) -> DbResult<()> {
        let pool = pool.clone();
        let app_id = app_id.to_string();
        tokio::task::spawn_blocking(move || {
            let mut conn = pool.get()?;
            diesel::delete(app_backups::table.filter(app_backups::app_id.eq(&app_id)))
                .execute(&mut conn)?;
            Ok(())
        })
        .await?
    }
}
