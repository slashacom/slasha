use std::{collections::HashMap, time::Duration};

use bollard::{
    Docker,
    models::{
        ContainerCreateBody, EndpointSettings, HostConfig, Mount, MountType, NetworkingConfig,
        RestartPolicy, RestartPolicyNameEnum,
    },
    query_parameters::{
        CreateContainerOptions, CreateImageOptions, RemoveContainerOptionsBuilder,
        WaitContainerOptions,
    },
};
use chrono::Utc;
use futures_util::StreamExt;
use slasha_db::{
    DbPool,
    app::App,
    cron::{CronJob, CronRun, CronRunStatus, CronRuntime},
    deployment::DeploymentStatus,
    logs::ResourceKind,
    repos::{app::AppRepo, cron::CronRunRepo, deployment::DeploymentRepo, node::NodeRepo},
};

use crate::{
    docker::{
        DockerResult,
        app::{deploy::context::MANAGED_DATA_PATH, env::resolve_app_env, image::image_tag},
        labels::cron_container_labels,
        log_driver::default_log_config,
        naming::{app_network_name, app_volume_name, cron_container_name},
        utils::{self, stream_container_logs},
    },
    logs::{LogBus, LogWriter},
    node_registry::NodeRegistry,
    proxy::container::PROXY_NETWORK_NAME,
};

/// Lightweight image used by utility crons so webhook/HTTP jobs (curl) work
/// without the app image needing those tools installed.
const UTILITY_IMAGE: &str = "curlimages/curl:latest";

/// Execution outcome of a cron container task.
enum CronOutcome {
    Completed { exit_code: i64 },
    TimedOut,
}

/// Options and contextual references for executing an ephemeral cron container.
struct RunCronContainerContext<'a> {
    docker: &'a Docker,
    log: &'a LogWriter,
    app: &'a App,
    image: &'a str,
    runtime: CronRuntime,
    cron_job_id: &'a str,
    cron_run_id: &'a str,
    command: &'a str,
    env_map: HashMap<String, String>,
    timeout_secs: u64,
}

/// Executes a scheduled or manually triggered cron job run in an ephemeral container.
///
/// # Arguments
///
/// * `db_pool` - Database connection pool ([`DbPool`]).
/// * `node_registry` - Node registry handle ([`NodeRegistry`]).
/// * `log_bus` - Application event log bus handle ([`LogBus`]).
/// * `job` - Cron job model ([`CronJob`]).
/// * `run` - Cron run tracking model ([`CronRun`]).
pub async fn run_cron_job(
    db_pool: DbPool,
    node_registry: NodeRegistry,
    log_bus: LogBus,
    job: CronJob,
    run: CronRun,
) {
    let run_id = run.id.clone();

    if let Err(err) = CronRunRepo::mark_running(&db_pool, &run_id, Utc::now().naive_utc()).await {
        tracing::error!(target: "slasha::cron", run = %run_id, error = ?err, "failed to mark cron run running");
        return;
    }

    let (status, exit_code, error) =
        match execute(&db_pool, &node_registry, &log_bus, &job, &run_id).await {
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
            Err(err) => (CronRunStatus::Failed, None, Some(err.to_string())),
        };

    if let Err(err) = CronRunRepo::mark_finished(&db_pool, &run_id, status, exit_code, error).await
    {
        tracing::error!(target: "slasha::cron", run = %run_id, error = ?err, "failed to mark cron run finished");
    }
}

/// Internal helper executing the cron job lifecycle and returning the resulting outcome.
///
/// # Arguments
///
/// * `db_pool` - Database connection pool ([`DbPool`]).
/// * `node_registry` - Node registry handle ([`NodeRegistry`]).
/// * `log_bus` - Application log bus handle ([`LogBus`]).
/// * `job` - Cron job model ([`CronJob`]).
/// * `run_id` - Target cron run ID string.
///
/// # Returns
///
/// An [`anyhow::Result`] containing the [`CronOutcome`].
async fn execute(
    db_pool: &DbPool,
    node_registry: &NodeRegistry,
    log_bus: &LogBus,
    job: &CronJob,
    run_id: &str,
) -> anyhow::Result<CronOutcome> {
    let app = AppRepo::find_by_id(db_pool, &job.app_id).await?;
    let node = NodeRepo::get(db_pool, &app.node_id).await?;

    let docker_client = node_registry.get_client(&node)?;

    let running_deployment = DeploymentRepo::list_active_for_app(db_pool, &job.app_id)
        .await?
        .into_iter()
        .find(|d| matches!(d.status, DeploymentStatus::Running));

    let image = match job.runtime {
        CronRuntime::App => {
            let deployment = running_deployment.as_ref().ok_or_else(|| {
                anyhow::anyhow!("no running deployment to run the command against")
            })?;
            image_tag(&app.slug, &deployment.id)
        }
        CronRuntime::Utility => {
            ensure_image(&docker_client, UTILITY_IMAGE).await?;
            UTILITY_IMAGE.to_string()
        }
    };

    let env_map = resolve_cron_env(db_pool, &app, running_deployment.is_some()).await?;

    let log_writer = log_bus.writer(ResourceKind::Cron, run_id).app_id(&app.id);

    let outcome = run_cron_container(RunCronContainerContext {
        docker: &docker_client,
        log: &log_writer,
        app: &app,
        image: &image,
        runtime: job.runtime,
        cron_job_id: &job.id,
        cron_run_id: run_id,
        command: &job.command,
        env_map,
        timeout_secs: job.timeout_secs.max(1) as u64,
    })
    .await?;

    Ok(outcome)
}

/// Resolves environment variables for a cron job execution based on active deployment availability.
///
/// # Arguments
///
/// * `db_pool` - Database connection pool ([`DbPool`]).
/// * `app` - Target application model ([`App`]).
/// * `has_active_deployment` - Whether an active deployment exists for service reference resolution.
///
/// # Returns
///
/// An [`anyhow::Result`] containing a [`HashMap`] of key-value environment variables.
async fn resolve_cron_env(
    db_pool: &DbPool,
    app: &App,
    has_active_deployment: bool,
) -> anyhow::Result<HashMap<String, String>> {
    if !has_active_deployment {
        let app_vars = AppRepo::get_env_vars(db_pool, &app.id).await?;

        return Ok(app_vars.into_iter().map(|v| (v.key, v.value)).collect());
    }

    let env_map = resolve_app_env(db_pool, app).await?;
    Ok(env_map)
}

/// Ensures a Docker image is present locally, pulling it if missing.
///
/// # Arguments
///
/// * `docker` - Docker API client ([`Docker`]).
/// * `image` - Target Docker image tag string.
///
/// # Returns
///
/// An [`anyhow::Result`] indicating image readiness.
async fn ensure_image(docker: &Docker, image: &str) -> anyhow::Result<()> {
    if docker.inspect_image(image).await.is_ok() {
        return Ok(());
    }

    let mut stream = docker.create_image(
        Some(CreateImageOptions {
            from_image: Some(image.to_string()),
            ..Default::default()
        }),
        None,
        None,
    );
    while let Some(item) = stream.next().await {
        item?;
    }
    Ok(())
}

/// Creates, streams logs from, and cleans up an ephemeral Docker container for a cron job.
///
/// # Arguments
///
/// * `ctx` - Execution context and parameter references ([`RunCronContainerContext`]).
///
/// # Returns
///
/// A [`DockerResult`](crate::docker::DockerResult) containing the [`CronOutcome`].
async fn run_cron_container(ctx: RunCronContainerContext<'_>) -> DockerResult<CronOutcome> {
    let container_name = cron_container_name(ctx.cron_run_id);

    ctx.log.stdout(format!("Running command: {}", ctx.command));

    let mounts = match ctx.runtime {
        CronRuntime::App => {
            let volume_name = app_volume_name(&ctx.app.id, MANAGED_DATA_PATH);
            utils::create_volume(ctx.docker, &volume_name, None).await?;

            Some(vec![Mount {
                typ: Some(MountType::VOLUME),
                source: Some(volume_name),
                target: Some(MANAGED_DATA_PATH.to_string()),
                ..Default::default()
            }])
        }
        CronRuntime::Utility => None,
    };

    let labels = cron_container_labels(ctx.app, ctx.cron_job_id, ctx.cron_run_id);

    let env: Option<Vec<String>> = if ctx.env_map.is_empty() {
        None
    } else {
        Some(
            ctx.env_map
                .into_iter()
                .map(|(k, v)| format!("{}={}", k, v))
                .collect(),
        )
    };

    let app_network = app_network_name(&ctx.app.id);
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

    ctx.docker
        .create_container(
            Some(CreateContainerOptions {
                name: Some(container_name.clone()),
                ..Default::default()
            }),
            ContainerCreateBody {
                image: Some(ctx.image.to_string()),
                labels: Some(labels),
                env,
                entrypoint: Some(vec!["sh".to_string(), "-c".to_string()]),
                cmd: Some(vec![ctx.command.to_string()]),
                host_config: Some(HostConfig {
                    restart_policy: Some(RestartPolicy {
                        name: Some(RestartPolicyNameEnum::EMPTY),
                        maximum_retry_count: None,
                    }),
                    mounts,
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

    utils::start_container(ctx.docker, &container_name).await?;

    let stream_handle =
        stream_container_logs(ctx.docker.clone(), ctx.log.clone(), container_name.clone());

    let wait = async {
        ctx.docker
            .wait_container(
                &container_name,
                Some(WaitContainerOptions {
                    condition: "not-running".to_string(),
                }),
            )
            .next()
            .await
    };

    let outcome = match tokio::time::timeout(Duration::from_secs(ctx.timeout_secs), wait).await {
        Ok(Some(Ok(res))) => CronOutcome::Completed {
            exit_code: res.status_code,
        },
        Ok(Some(Err(err))) => {
            ctx.log
                .stderr(format!("Error while waiting for container: {}", err));
            CronOutcome::Completed { exit_code: -1 }
        }
        Ok(None) => CronOutcome::Completed { exit_code: -1 },
        Err(_) => {
            ctx.log.stderr(format!(
                "Command exceeded timeout of {}s; terminating",
                ctx.timeout_secs
            ));
            CronOutcome::TimedOut
        }
    };

    if let Err(err) = ctx
        .docker
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
