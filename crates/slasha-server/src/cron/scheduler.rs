use std::{sync::Arc, time::Duration};

use bollard::Docker;
use chrono::Utc;
use slasha_db::{
    DbPool,
    cron::{CronJob, CronRunStatus, CronRunTrigger},
    repos::cron::{CronJobRepo, CronRunRepo, new_run},
};
use tokio::time::sleep;
use tracing::{error, info, warn};

use super::{runner, schedule};
use crate::docker::logs::LogManager;

const TICK_INTERVAL: Duration = Duration::from_secs(30);

pub fn spawn_cron_scheduler(db_pool: DbPool, docker: Docker, log_manager: Arc<LogManager>) {
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
            if let Err(err) = tick(&db_pool, &docker, &log_manager).await {
                error!(target: "slasha::cron", error = ?err, "cron scheduler tick failed");
            }
            sleep(TICK_INTERVAL).await;
        }
    });
}

async fn tick(
    db_pool: &DbPool,
    docker: &Docker,
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

        let run = CronRunRepo::create(db_pool, new_run(&job.id, CronRunTrigger::Scheduled)).await?;

        let db_pool = db_pool.clone();
        let docker = docker.clone();
        let log_manager = log_manager.clone();
        tokio::spawn(async move {
            runner::run_cron_job(db_pool, docker, log_manager, job, run).await;
        });
    }

    Ok(())
}

async fn record_skipped(db_pool: &DbPool, job: &CronJob) {
    let mut run = new_run(&job.id, CronRunTrigger::Scheduled);
    run.status = CronRunStatus::Skipped;
    run.finished_at = Some(Utc::now().naive_utc());
    if let Err(err) = CronRunRepo::create(db_pool, run).await {
        warn!(target: "slasha::cron", job = %job.id, error = ?err, "failed to record skipped run");
    }
}
