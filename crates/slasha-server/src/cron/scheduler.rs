use std::{sync::Arc, time::Duration};

use chrono::Utc;
use slasha_db::{
    DbPool,
    cron::{CronJob, CronRunStatus, CronRunTrigger, NewCronRun},
    repos::cron::{CronJobRepo, CronRunRepo},
};
use tokio::time::sleep;
use tracing::{error, info, warn};

use super::{runner, schedule};
use crate::{docker::DockerRegistry, logs::LogManager};

const TICK_INTERVAL: Duration = Duration::from_secs(30);

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

async fn tick(
    db_pool: &DbPool,
    docker_registry: &DockerRegistry,
    log_manager: &Arc<LogManager>,
) -> anyhow::Result<()> {
    let jobs = CronJobRepo::list_enabled(db_pool).await?;
    let now = Utc::now();

    for job in jobs {
        let tz = schedule::parse_timezone(&job.timezone).unwrap_or(chrono_tz::UTC);
        let parsed = match schedule::parse(&job.schedule) {
            Ok(parsed) => parsed,
            Err(err) => {
                warn!(target: "slasha::cron", job = %job.id, error = %err, "invalid cron schedule; skipping");
                continue;
            }
        };

        let next_run_at = match job.next_run_at {
            Some(next) => next,
            None => {
                let next = parsed.next_after(now, tz).map(|dt| dt.naive_utc());
                CronJobRepo::update_schedule_state(db_pool, &job.id, job.last_run_at, next).await?;
                continue;
            }
        };

        if next_run_at > now.naive_utc() {
            continue;
        }

        // Advance the schedule before firing so a slow run never double-fires.
        let following = parsed.next_after(now, tz).map(|dt| dt.naive_utc());
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

        let db_pool = db_pool.clone();
        let docker_registry = docker_registry.clone();
        let log_manager = log_manager.clone();
        tokio::spawn(async move {
            runner::run_cron_job(db_pool, docker_registry, log_manager, job, run).await;
        });
    }

    Ok(())
}

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
