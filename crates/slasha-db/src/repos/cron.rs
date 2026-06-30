use chrono::{NaiveDateTime, Utc};
use diesel::prelude::*;

use crate::{
    connection::DbPool,
    error::{DbError, DbResult},
    models::{
        cron::{CronJob, CronRun, CronRunStatus, CronRunTrigger},
        schema::{cron_jobs, cron_runs},
    },
};

pub struct CronJobRepo;

impl CronJobRepo {
    pub async fn list_for_app(pool: &DbPool, app_id: &str) -> DbResult<Vec<CronJob>> {
        let pool = pool.clone();
        let app_id = app_id.to_string();
        tokio::task::spawn_blocking(move || {
            let mut conn = pool.get()?;
            Ok(cron_jobs::table
                .filter(cron_jobs::app_id.eq(&app_id))
                .order(cron_jobs::created_at.desc())
                .load::<CronJob>(&mut conn)?)
        })
        .await?
    }

    pub async fn list_all(pool: &DbPool) -> DbResult<Vec<CronJob>> {
        let pool = pool.clone();
        tokio::task::spawn_blocking(move || {
            let mut conn = pool.get()?;
            Ok(cron_jobs::table
                .order(cron_jobs::created_at.desc())
                .load::<CronJob>(&mut conn)?)
        })
        .await?
    }

    pub async fn list_enabled(pool: &DbPool) -> DbResult<Vec<CronJob>> {
        let pool = pool.clone();
        tokio::task::spawn_blocking(move || {
            let mut conn = pool.get()?;
            Ok(cron_jobs::table
                .filter(cron_jobs::enabled.eq(true))
                .order(cron_jobs::created_at.asc())
                .load::<CronJob>(&mut conn)?)
        })
        .await?
    }

    pub async fn find(pool: &DbPool, id: &str, app_id: &str) -> DbResult<CronJob> {
        let pool = pool.clone();
        let id = id.to_string();
        let app_id = app_id.to_string();
        tokio::task::spawn_blocking(move || {
            let mut conn = pool.get()?;
            cron_jobs::table
                .filter(cron_jobs::id.eq(&id))
                .filter(cron_jobs::app_id.eq(&app_id))
                .first::<CronJob>(&mut conn)
                .optional()?
                .ok_or_else(|| DbError::NotFound(format!("cron job '{}' not found", id)))
        })
        .await?
    }

    pub async fn find_by_id(pool: &DbPool, id: &str) -> DbResult<CronJob> {
        let pool = pool.clone();
        let id = id.to_string();
        tokio::task::spawn_blocking(move || {
            let mut conn = pool.get()?;
            cron_jobs::table
                .filter(cron_jobs::id.eq(&id))
                .first::<CronJob>(&mut conn)
                .optional()?
                .ok_or_else(|| DbError::NotFound(format!("cron job '{}' not found", id)))
        })
        .await?
    }

    pub async fn create(pool: &DbPool, job: CronJob) -> DbResult<CronJob> {
        let pool = pool.clone();
        tokio::task::spawn_blocking(move || {
            let mut conn = pool.get()?;
            diesel::insert_into(cron_jobs::table)
                .values(&job)
                .execute(&mut conn)?;
            Ok(job)
        })
        .await?
    }

    pub async fn update(pool: &DbPool, id: &str, job: CronJob) -> DbResult<CronJob> {
        let pool = pool.clone();
        let id = id.to_string();
        tokio::task::spawn_blocking(move || {
            let mut conn = pool.get()?;
            let updated_at = Utc::now().naive_utc();

            diesel::update(cron_jobs::table.filter(cron_jobs::id.eq(&id)))
                .set((
                    cron_jobs::name.eq(&job.name),
                    cron_jobs::schedule.eq(&job.schedule),
                    cron_jobs::command.eq(&job.command),
                    cron_jobs::timezone.eq(&job.timezone),
                    cron_jobs::enabled.eq(job.enabled),
                    cron_jobs::timeout_secs.eq(job.timeout_secs),
                    cron_jobs::runtime.eq(job.runtime),
                    cron_jobs::next_run_at.eq(job.next_run_at),
                    cron_jobs::updated_at.eq(updated_at),
                ))
                .execute(&mut conn)?;

            cron_jobs::table
                .filter(cron_jobs::id.eq(&id))
                .first::<CronJob>(&mut conn)
                .optional()?
                .ok_or_else(|| DbError::NotFound(format!("cron job '{}' not found", id)))
        })
        .await?
    }

    pub async fn update_schedule_state(
        pool: &DbPool,
        id: &str,
        last_run_at: Option<NaiveDateTime>,
        next_run_at: Option<NaiveDateTime>,
    ) -> DbResult<()> {
        let pool = pool.clone();
        let id = id.to_string();
        tokio::task::spawn_blocking(move || {
            let mut conn = pool.get()?;
            diesel::update(cron_jobs::table.filter(cron_jobs::id.eq(&id)))
                .set((
                    cron_jobs::last_run_at.eq(last_run_at),
                    cron_jobs::next_run_at.eq(next_run_at),
                ))
                .execute(&mut conn)?;
            Ok(())
        })
        .await?
    }

    pub async fn delete(pool: &DbPool, id: &str, app_id: &str) -> DbResult<usize> {
        let pool = pool.clone();
        let id = id.to_string();
        let app_id = app_id.to_string();
        tokio::task::spawn_blocking(move || {
            let mut conn = pool.get()?;
            Ok(diesel::delete(
                cron_jobs::table
                    .filter(cron_jobs::id.eq(&id))
                    .filter(cron_jobs::app_id.eq(&app_id)),
            )
            .execute(&mut conn)?)
        })
        .await?
    }
}

pub struct CronRunRepo;

impl CronRunRepo {
    pub async fn list_for_job(
        pool: &DbPool,
        cron_job_id: &str,
        limit: i64,
    ) -> DbResult<Vec<CronRun>> {
        let pool = pool.clone();
        let cron_job_id = cron_job_id.to_string();
        tokio::task::spawn_blocking(move || {
            let mut conn = pool.get()?;
            Ok(cron_runs::table
                .filter(cron_runs::cron_job_id.eq(&cron_job_id))
                .order(cron_runs::created_at.desc())
                .limit(limit)
                .load::<CronRun>(&mut conn)?)
        })
        .await?
    }

    pub async fn list_ids_for_job(pool: &DbPool, cron_job_id: &str) -> DbResult<Vec<String>> {
        let pool = pool.clone();
        let cron_job_id = cron_job_id.to_string();
        tokio::task::spawn_blocking(move || {
            let mut conn = pool.get()?;
            Ok(cron_runs::table
                .filter(cron_runs::cron_job_id.eq(&cron_job_id))
                .select(cron_runs::id)
                .load::<String>(&mut conn)?)
        })
        .await?
    }

    pub async fn find(pool: &DbPool, id: &str, cron_job_id: &str) -> DbResult<CronRun> {
        let pool = pool.clone();
        let id = id.to_string();
        let cron_job_id = cron_job_id.to_string();
        tokio::task::spawn_blocking(move || {
            let mut conn = pool.get()?;
            cron_runs::table
                .filter(cron_runs::id.eq(&id))
                .filter(cron_runs::cron_job_id.eq(&cron_job_id))
                .first::<CronRun>(&mut conn)
                .optional()?
                .ok_or_else(|| DbError::NotFound(format!("cron run '{}' not found", id)))
        })
        .await?
    }

    pub async fn latest_for_job(pool: &DbPool, cron_job_id: &str) -> DbResult<Option<CronRun>> {
        let pool = pool.clone();
        let cron_job_id = cron_job_id.to_string();
        tokio::task::spawn_blocking(move || {
            let mut conn = pool.get()?;
            Ok(cron_runs::table
                .filter(cron_runs::cron_job_id.eq(&cron_job_id))
                .order(cron_runs::created_at.desc())
                .first::<CronRun>(&mut conn)
                .optional()?)
        })
        .await?
    }

    pub async fn latest_outcome_for_job(
        pool: &DbPool,
        cron_job_id: &str,
    ) -> DbResult<Option<CronRun>> {
        let pool = pool.clone();
        let cron_job_id = cron_job_id.to_string();
        tokio::task::spawn_blocking(move || {
            let mut conn = pool.get()?;
            Ok(cron_runs::table
                .filter(cron_runs::cron_job_id.eq(&cron_job_id))
                .filter(
                    cron_runs::status
                        .eq(CronRunStatus::Succeeded)
                        .or(cron_runs::status.eq(CronRunStatus::Failed))
                        .or(cron_runs::status.eq(CronRunStatus::TimedOut)),
                )
                .order(cron_runs::created_at.desc())
                .first::<CronRun>(&mut conn)
                .optional()?)
        })
        .await?
    }

    pub async fn has_active(pool: &DbPool, cron_job_id: &str) -> DbResult<bool> {
        let pool = pool.clone();
        let cron_job_id = cron_job_id.to_string();
        tokio::task::spawn_blocking(move || {
            let mut conn = pool.get()?;
            let count: i64 = cron_runs::table
                .filter(cron_runs::cron_job_id.eq(&cron_job_id))
                .filter(
                    cron_runs::status
                        .eq(CronRunStatus::Pending)
                        .or(cron_runs::status.eq(CronRunStatus::Running)),
                )
                .count()
                .get_result(&mut conn)?;
            Ok(count > 0)
        })
        .await?
    }

    /// Mark any runs left mid-flight as failed. Used on startup to clear runs
    /// whose container died with the server, since a stuck Pending/Running row
    /// would otherwise block the job forever via `has_active`.
    pub async fn fail_interrupted(pool: &DbPool) -> DbResult<usize> {
        let pool = pool.clone();
        tokio::task::spawn_blocking(move || {
            let mut conn = pool.get()?;
            let finished_at = Utc::now().naive_utc();
            Ok(diesel::update(
                cron_runs::table.filter(
                    cron_runs::status
                        .eq(CronRunStatus::Pending)
                        .or(cron_runs::status.eq(CronRunStatus::Running)),
                ),
            )
            .set((
                cron_runs::status.eq(CronRunStatus::Failed),
                cron_runs::error.eq(Some("Run interrupted by a server restart".to_string())),
                cron_runs::finished_at.eq(Some(finished_at)),
            ))
            .execute(&mut conn)?)
        })
        .await?
    }

    pub async fn create(pool: &DbPool, run: CronRun) -> DbResult<CronRun> {
        let pool = pool.clone();
        tokio::task::spawn_blocking(move || {
            let mut conn = pool.get()?;
            diesel::insert_into(cron_runs::table)
                .values(&run)
                .execute(&mut conn)?;
            Ok(run)
        })
        .await?
    }

    pub async fn mark_running(pool: &DbPool, id: &str, started_at: NaiveDateTime) -> DbResult<()> {
        let pool = pool.clone();
        let id = id.to_string();
        tokio::task::spawn_blocking(move || {
            let mut conn = pool.get()?;
            diesel::update(cron_runs::table.filter(cron_runs::id.eq(&id)))
                .set((
                    cron_runs::status.eq(CronRunStatus::Running),
                    cron_runs::started_at.eq(Some(started_at)),
                ))
                .execute(&mut conn)?;
            Ok(())
        })
        .await?
    }

    pub async fn mark_finished(
        pool: &DbPool,
        id: &str,
        status: CronRunStatus,
        exit_code: Option<i32>,
        error: Option<String>,
    ) -> DbResult<()> {
        let pool = pool.clone();
        let id = id.to_string();
        tokio::task::spawn_blocking(move || {
            let mut conn = pool.get()?;
            let finished_at = Utc::now().naive_utc();
            diesel::update(cron_runs::table.filter(cron_runs::id.eq(&id)))
                .set((
                    cron_runs::status.eq(status),
                    cron_runs::exit_code.eq(exit_code),
                    cron_runs::error.eq(error),
                    cron_runs::finished_at.eq(Some(finished_at)),
                ))
                .execute(&mut conn)?;
            Ok(())
        })
        .await?
    }
}

pub fn new_run(cron_job_id: &str, trigger_kind: CronRunTrigger) -> CronRun {
    CronRun {
        id: uuid::Uuid::new_v4().to_string(),
        cron_job_id: cron_job_id.to_string(),
        status: CronRunStatus::Pending,
        trigger_kind,
        exit_code: None,
        error: None,
        started_at: None,
        finished_at: None,
        created_at: Utc::now().naive_utc(),
    }
}
