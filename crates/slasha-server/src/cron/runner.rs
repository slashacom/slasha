use std::{collections::HashMap, sync::Arc, time::Duration};

use bollard::{
    Docker,
    models::{
        ContainerCreateBody, EndpointSettings, HostConfig, Mount, MountTypeEnum, NetworkingConfig,
        RestartPolicy, RestartPolicyNameEnum, VolumeCreateRequest,
    },
    query_parameters::{
        CreateContainerOptions, RemoveContainerOptionsBuilder, StartContainerOptionsBuilder,
        WaitContainerOptions,
    },
};
use chrono::Utc;
use futures_util::StreamExt;
use slasha_db::{
    DbPool,
    app::App,
    cron::{CronJob, CronRun, CronRunStatus},
    deployment::{Deployment, DeploymentStatus},
    repos::{app::AppRepo, cron::CronRunRepo, deployment::DeploymentRepo, service::ServiceRepo},
};

use crate::{
    docker::{
        deployment::{container::MANAGED_DATA_PATH, executor::resolve_app_env},
        image_tag,
        log_driver::default_log_config,
        logs::{LogHandle, LogKey, LogManager, stream_container_logs},
        naming::{app_network_name, app_volume_name},
    },
    proxy::container::PROXY_NETWORK_NAME,
};

enum CronOutcome {
    Completed { exit_code: i64 },
    TimedOut,
}

fn cron_container_name(run_id: &str) -> String {
    format!("slasha-cron-{}", run_id)
}

pub async fn run_cron_job(
    db_pool: DbPool,
    docker: Docker,
    log_manager: Arc<LogManager>,
    job: CronJob,
    run: CronRun,
) {
    let run_id = run.id.clone();

    if let Err(err) = CronRunRepo::mark_running(&db_pool, &run_id, Utc::now().naive_utc()).await {
        tracing::error!(target: "slasha::cron", run = %run_id, error = ?err, "failed to mark cron run running");
        return;
    }

    let (status, exit_code, error) =
        match execute(&db_pool, &docker, &log_manager, &job, &run_id).await {
            Ok(CronOutcome::Completed { exit_code }) => {
                let status = if exit_code == 0 {
                    CronRunStatus::Succeeded
                } else {
                    CronRunStatus::Failed
                };
                (status, Some(exit_code as i32), None)
            }
            Ok(CronOutcome::TimedOut) => (
                CronRunStatus::TimedOut,
                None,
                Some(format!("run exceeded timeout of {}s", job.timeout_secs)),
            ),
            Err(message) => (CronRunStatus::Failed, None, Some(message)),
        };

    if let Err(err) = CronRunRepo::mark_finished(&db_pool, &run_id, status, exit_code, error).await
    {
        tracing::error!(target: "slasha::cron", run = %run_id, error = ?err, "failed to mark cron run finished");
    }
}

async fn execute(
    db_pool: &DbPool,
    docker: &Docker,
    log_manager: &Arc<LogManager>,
    job: &CronJob,
    run_id: &str,
) -> Result<CronOutcome, String> {
    let app = AppRepo::find_by_id(db_pool, &job.app_id)
        .await
        .map_err(|e| e.to_string())?;

    let deployment = DeploymentRepo::list_active_for_app(db_pool, &job.app_id)
        .await
        .map_err(|e| e.to_string())?
        .into_iter()
        .find(|d| matches!(d.status, DeploymentStatus::Running))
        .ok_or_else(|| "no running deployment to run the command against".to_string())?;

    let app_vars = AppRepo::get_env_vars(db_pool, &app.id)
        .await
        .map_err(|e| e.to_string())?;
    let services = ServiceRepo::list_for_app(db_pool, &app.id)
        .await
        .map_err(|e| e.to_string())?;
    let env_map = resolve_app_env(db_pool, &app, &deployment, app_vars, services)
        .await
        .map_err(|e| e.to_string())?;

    let log = log_manager
        .get_logger(&LogKey::Cron {
            app_slug: app.slug.clone(),
            cron_run_id: run_id.to_string(),
        })
        .await
        .map_err(|e| e.to_string())?;

    run_cron_container(
        docker,
        &log,
        &app,
        &deployment,
        &job.id,
        run_id,
        &job.command,
        env_map,
        job.timeout_secs.max(1) as u64,
    )
    .await
    .map_err(|e| e.to_string())
}

async fn run_cron_container(
    docker: &Docker,
    log: &LogHandle,
    app: &App,
    deployment: &Deployment,
    cron_job_id: &str,
    cron_run_id: &str,
    command: &str,
    env_map: HashMap<String, String>,
    timeout_secs: u64,
) -> crate::docker::DeploymentResult<CronOutcome> {
    let container_name = cron_container_name(cron_run_id);

    log.send(format!("Running command: {}", command)).await?;

    let volume_name = app_volume_name(&app.id, MANAGED_DATA_PATH);
    docker
        .create_volume(VolumeCreateRequest {
            name: Some(volume_name.clone()),
            ..Default::default()
        })
        .await?;
    let mounts = vec![Mount {
        typ: Some(MountTypeEnum::VOLUME),
        source: Some(volume_name),
        target: Some(MANAGED_DATA_PATH.to_string()),
        ..Default::default()
    }];

    let mut labels: HashMap<String, String> = HashMap::new();
    labels.insert("slasha.managed".into(), "true".into());
    labels.insert("slasha.app_id".into(), app.id.clone());
    labels.insert("slasha.app_slug".into(), app.slug.clone());
    labels.insert("slasha.cron_job_id".into(), cron_job_id.to_string());
    labels.insert("slasha.cron_run_id".into(), cron_run_id.to_string());
    labels.insert("slasha.process_type".into(), "cron".into());

    let env: Option<Vec<String>> = if env_map.is_empty() {
        None
    } else {
        Some(
            env_map
                .into_iter()
                .map(|(k, v)| format!("{}={}", k, v))
                .collect(),
        )
    };

    let app_network = app_network_name(&app.id);
    let mut endpoints_config = HashMap::new();
    endpoints_config.insert(
        app_network.clone(),
        EndpointSettings {
            network_id: Some(app_network),
            ..Default::default()
        },
    );
    endpoints_config.insert(
        PROXY_NETWORK_NAME.to_string(),
        EndpointSettings {
            network_id: Some(PROXY_NETWORK_NAME.to_string()),
            ..Default::default()
        },
    );

    docker
        .create_container(
            Some(CreateContainerOptions {
                name: Some(container_name.clone()),
                ..Default::default()
            }),
            ContainerCreateBody {
                image: Some(image_tag(&app.slug, &deployment.commit_sha)),
                labels: Some(labels),
                env,
                cmd: Some(vec![
                    "sh".to_string(),
                    "-c".to_string(),
                    command.to_string(),
                ]),
                host_config: Some(HostConfig {
                    restart_policy: Some(RestartPolicy {
                        name: Some(RestartPolicyNameEnum::EMPTY),
                        maximum_retry_count: None,
                    }),
                    mounts: Some(mounts),
                    log_config: Some(default_log_config()),
                    ..Default::default()
                }),
                networking_config: Some(NetworkingConfig {
                    endpoints_config: Some(endpoints_config),
                }),
                ..Default::default()
            },
        )
        .await?;

    docker
        .start_container(
            &container_name,
            Some(StartContainerOptionsBuilder::new().build()),
        )
        .await?;

    let stream_handle = stream_container_logs(
        docker.clone(),
        log.clone(),
        container_name.clone(),
        Some("[cron]".to_string()),
    );

    let wait = async {
        docker
            .wait_container(
                &container_name,
                Some(WaitContainerOptions {
                    condition: "not-running".to_string(),
                }),
            )
            .next()
            .await
    };

    let outcome = match tokio::time::timeout(Duration::from_secs(timeout_secs), wait).await {
        Ok(Some(Ok(res))) => CronOutcome::Completed {
            exit_code: res.status_code,
        },
        Ok(Some(Err(err))) => {
            log.send(format!("Error while waiting for container: {}", err))
                .await?;
            CronOutcome::Completed { exit_code: -1 }
        }
        Ok(None) => CronOutcome::Completed { exit_code: -1 },
        Err(_) => {
            log.send(format!(
                "Command exceeded timeout of {}s; terminating",
                timeout_secs
            ))
            .await?;
            CronOutcome::TimedOut
        }
    };

    // Remove the container first so the follow log stream terminates, then wait
    // for the stream task to flush any remaining output.
    if let Err(err) = docker
        .remove_container(
            &container_name,
            Some(RemoveContainerOptionsBuilder::new().force(true).build()),
        )
        .await
    {
        tracing::warn!(container = %container_name, error = ?err, "Failed to remove cron container");
    }
    let _ = stream_handle.await;

    Ok(outcome)
}
