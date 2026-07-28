use std::{sync::Arc, time::Duration};

use chrono::Utc;
use slasha_db::{
    DbPool,
    cron::{CronJob, CronRunStatus, CronRunTrigger, NewCronRun},
    repos::cron::{CronJobRepo, CronRunRepo},
};
use tokio::time::sleep;
use tracing::{error, info, warn};

use crate::{
    docker::{DockerRegistry, cron::run_cron_job},
    logs::LogManager,
};

const TICK_INTERVAL: Duration = Duration::from_secs(30);

/// Spawns the background cron scheduler tick loop.
///
/// # Arguments
///
/// * `db_pool` - Database connection pool ([`DbPool`]).
/// * `docker_registry` - Node Docker client registry ([`DockerRegistry`]).
/// * `log_manager` - Application log manager handle ([`LogManager`]).
pub fn spawn_cron_scheduler(
    db_pool: DbPool,
    docker_registry: DockerRegistry,
    log_manager: Arc<LogManager>,
) {
    tokio::spawn(async move {
        info!(target: "slasha::cron", "cron scheduler started");
        match CronRunRepo::fail_interrupted(&db_pool).await {
            Ok(count) if count > 0 => {
                warn!(target: "slasha::cron", count, "marked interrupted cron runs as failed")
            }
            Ok(_) => {}
            Err(err) => {
                error!(target: "slasha::cron", error = ?err, "failed to reconcile interrupted cron runs")
            }
        }
        loop {
            if let Err(err) = tick(&db_pool, &docker_registry, &log_manager).await {
                error!(target: "slasha::cron", error = ?err, "cron scheduler tick failed");
            }
            sleep(TICK_INTERVAL).await;
        }
    });
}

/// Evaluates active cron jobs on a periodic scheduler tick and dispatches due executions.
///
/// # Arguments
///
/// * `db_pool` - Database connection pool ([`DbPool`]).
/// * `docker_registry` - Node Docker client registry ([`DockerRegistry`]).
/// * `log_manager` - Application log manager handle ([`LogManager`]).
///
/// # Returns
///
/// An [`anyhow::Result`] indicating tick processing success.
async fn tick(
    db_pool: &DbPool,
    docker_registry: &DockerRegistry,
    log_manager: &Arc<LogManager>,
) -> anyhow::Result<()> {
    let jobs = CronJobRepo::list_enabled(db_pool).await?;
    let now = Utc::now();

    for job in jobs {
        let next_run_at = match job.next_run_at {
            Some(next) => next,
            None => {
                let next = super::next_run_at(&job.schedule, &job.timezone, &now)
                    .ok()
                    .flatten();
                CronJobRepo::update_schedule_state(db_pool, &job.id, job.last_run_at, next).await?;
                continue;
            }
        };

        if next_run_at > now.naive_utc() {
            continue;
        }

        // Advance the schedule before firing so a slow run never double-fires.
        let following = super::next_run_at(&job.schedule, &job.timezone, &now)
            .ok()
            .flatten();
        CronJobRepo::update_schedule_state(db_pool, &job.id, Some(now.naive_utc()), following)
            .await?;

        if CronRunRepo::has_active(db_pool, &job.id)
            .await
            .unwrap_or(false)
        {
            record_skipped(db_pool, &job).await;
            continue;
        }

        let new_run_data = NewCronRun {
            cron_job_id: job.id.clone(),
            status: CronRunStatus::Pending,
            trigger_kind: CronRunTrigger::Scheduled,
        };

        let run = CronRunRepo::create(db_pool, new_run_data).await?;

        tokio::spawn({
            let db_pool = db_pool.clone();
            let docker_registry = docker_registry.clone();
            let log_manager = log_manager.clone();

            async move {
                run_cron_job(db_pool, docker_registry, log_manager, job, run).await;
            }
        });
    }

    Ok(())
}

/// Records a skipped run entry in the database when a cron job execution overlaps an active run.
///
/// # Arguments
///
/// * `db_pool` - Database connection pool ([`DbPool`]).
/// * `job` - Target cron job model ([`CronJob`]).
async fn record_skipped(db_pool: &DbPool, job: &CronJob) {
    let new_run_data = NewCronRun {
        cron_job_id: job.id.clone(),
        status: CronRunStatus::Skipped,
        trigger_kind: CronRunTrigger::Scheduled,
    };

    match CronRunRepo::create(db_pool, new_run_data).await {
        Ok(run) => {
            let _ =
                CronRunRepo::mark_finished(db_pool, &run.id, CronRunStatus::Skipped, None, None)
                    .await;
        }
        Err(err) => {
            warn!(target: "slasha::cron", job = %job.id, error = ?err, "failed to record skipped run");
        }
    }
}
