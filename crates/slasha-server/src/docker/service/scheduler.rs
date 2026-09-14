use std::time::Duration;

use anyhow::Result;
use chrono::Utc;
use slasha_db::{
    repos::{app::AppRepo, service::ServiceRepo, service_backup::ServiceBackupRepo},
    service_backup::ServiceBackupTrigger,
};
use tokio::time::sleep;
use tracing::{error, info};

use crate::{cron::next_run_at, docker::service::ServiceDocker, state::AppState};

const TICK_INTERVAL: Duration = Duration::from_secs(30);

/// Spawns the background service backup scheduler tick loop.
///
/// # Arguments
///
/// * `state` - Global application state ([`AppState`]).
pub fn spawn_service_backup_scheduler(state: AppState) {
    tokio::spawn(async move {
        info!(target: "slasha::service_backup", "service backup scheduler started");
        loop {
            if let Err(err) = tick(&state).await {
                error!(target: "slasha::service_backup", error = ?err, "service backup scheduler tick failed");
            }
            sleep(TICK_INTERVAL).await;
        }
    });
}

/// Evaluates due service backup configs and triggers scheduled backups.
///
/// # Arguments
///
/// * `state` - Application state handle ([`AppState`]).
///
/// # Returns
///
/// An [`anyhow::Result`] indicating tick completion success.
async fn tick(state: &AppState) -> Result<()> {
    let db_pool = &state.storage.db_pool;
    let now = Utc::now();
    let configs = ServiceBackupRepo::list_due_configs(db_pool, now.naive_utc()).await?;

    for config in configs {
        let scheduled_run = match config.next_run_at {
            Some(next) => next,
            None => {
                let next = match next_run_at(&config.schedule, &config.timezone, &now) {
                    Ok(n) => Some(n),
                    Err(e) => {
                        error!(
                            target: "slasha::service_backup",
                            service_id = %config.service_id,
                            schedule = %config.schedule,
                            timezone = %config.timezone,
                            error = ?e,
                            "failed to compute initial next_run_at for backup schedule"
                        );
                        None
                    }
                };
                ServiceBackupRepo::update_schedule_state(
                    db_pool,
                    &config.service_id,
                    config.last_run_at,
                    next,
                )
                .await?;
                continue;
            }
        };

        if scheduled_run > now.naive_utc() {
            continue;
        }

        let following = match next_run_at(&config.schedule, &config.timezone, &now) {
            Ok(n) => Some(n),
            Err(e) => {
                error!(
                    target: "slasha::service_backup",
                    service_id = %config.service_id,
                    schedule = %config.schedule,
                    timezone = %config.timezone,
                    error = ?e,
                    "failed to compute next_run_at for backup schedule; retaining previous next_run_at"
                );
                config.next_run_at
            }
        };

        ServiceBackupRepo::update_schedule_state(
            db_pool,
            &config.service_id,
            Some(now.naive_utc()),
            following,
        )
        .await?;

        let service = match ServiceRepo::find_by_id(db_pool, &config.service_id).await {
            Ok(s) => s,
            Err(e) => {
                error!(
                    target: "slasha::service_backup",
                    service_id = %config.service_id,
                    error = ?e,
                    "service not found for backup"
                );
                continue;
            }
        };

        let app = match AppRepo::find_by_id(db_pool, &service.app_id).await {
            Ok(a) => a,
            Err(e) => {
                error!(
                    target: "slasha::service_backup",
                    app_id = %service.app_id,
                    error = ?e,
                    "app not found for service backup"
                );
                continue;
            }
        };

        let state = state.clone();
        tokio::spawn(async move {
            let service_docker = match ServiceDocker::new(state, app).await {
                Ok(sd) => sd,
                Err(e) => {
                    error!(
                        target: "slasha::service_backup",
                        service_id = %service.id,
                        error = ?e,
                        "failed to initialize ServiceDocker"
                    );
                    return;
                }
            };

            if let Err(e) = service_docker
                .run_service_backup(&service.id, ServiceBackupTrigger::Scheduled)
                .await
            {
                error!(
                    target: "slasha::service_backup",
                    service_id = %service.id,
                    error = ?e,
                    "scheduled service backup failed"
                );
            }
        });
    }

    Ok(())
}
