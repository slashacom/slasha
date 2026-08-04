use std::collections::HashMap;

use bollard::{Docker, plugin::ContainerSummaryStateEnum, query_parameters::ListContainersOptions};
pub use slasha_db::models::app_scale::ProcessContainer as ProcessContainerInfo;
use slasha_db::{
    app::App,
    deployment::Deployment,
    logs::{LogPrefix, ResourceKind},
    models::app_scale::{ProcessStatus, ProcessType},
};

use crate::{
    docker::{
        DockerError, DockerResult,
        labels::{LABEL_APP_ID, LABEL_DEPLOYMENT_ID, LABEL_INSTANCE_INDEX, LABEL_PROCESS_TYPE},
        process_container_name,
        utils::{self, stream_container_logs},
    },
    logs::LogWriter,
    state::Runtime,
};

/// Lists all Docker containers associated with a specific application deployment.
///
/// # Arguments
///
/// * `docker_client` - Docker API client ([`Docker`]).
/// * `deployment_id` - Deployment ID string.
///
/// # Returns
///
/// A [`DockerResult`] containing a vector of [`ProcessContainerInfo`]s.
pub async fn list_deployment_processes(
    docker_client: &Docker,
    deployment_id: &str,
) -> DockerResult<Vec<ProcessContainerInfo>> {
    let mut filters = HashMap::new();
    filters.insert(
        "label".to_string(),
        vec![format!("{}={}", LABEL_DEPLOYMENT_ID, deployment_id)],
    );

    let containers = docker_client
        .list_containers(Some(ListContainersOptions {
            all: true,
            filters: Some(filters),
            ..Default::default()
        }))
        .await?;

    let processes = containers
        .into_iter()
        .filter_map(|c| {
            let name = c
                .names
                .and_then(|n| n.into_iter().next())
                .map(|n| n.trim_start_matches('/').to_string())?;

            let labels = c.labels.unwrap_or_default();
            let process_type = labels
                .get(LABEL_PROCESS_TYPE)
                .and_then(|s| std::str::FromStr::from_str(s).ok())?;
            let instance_index = labels
                .get(LABEL_INSTANCE_INDEX)
                .and_then(|s| s.parse::<u32>().ok())?;

            let status = match c.state {
                Some(ContainerSummaryStateEnum::RUNNING) => ProcessStatus::Running,
                _ => ProcessStatus::Stopped,
            };

            Some(ProcessContainerInfo {
                name,
                process_type,
                instance_index,
                status,
            })
        })
        .collect();

    Ok(processes)
}

/// Starts an existing process container and attaches log streaming to the provided handle.
///
/// # Arguments
///
/// * `docker_client` - Docker API client ([`Docker`]).
/// * `log` - Log writer for output streaming ([`LogWriter`]).
/// * `app` - Target application model ([`App`]).
/// * `deployment` - Target deployment model ([`Deployment`]).
/// * `process_type` - Process type enum ([`ProcessType`]).
/// * `instance_index` - Replica instance index.
pub async fn start_process_container(
    docker_client: &Docker,
    log: &LogWriter,
    app: &App,
    deployment: &Deployment,
    process_type: ProcessType,
    instance_index: u32,
) -> DockerResult<()> {
    let container_name =
        process_container_name(&app.id, &deployment.id, &process_type, instance_index);

    let log_prefix = match process_type {
        ProcessType::Web => LogPrefix::Web(instance_index),
        ProcessType::Worker => LogPrefix::Worker(instance_index),
        _ => LogPrefix::System,
    };
    let log = log.clone().prefix(log_prefix);

    utils::start_container(docker_client, &container_name).await?;

    log.stdout(format!("Container {} started", container_name));

    stream_container_logs(docker_client.clone(), log, container_name);

    Ok(())
}

/// Checks whether any web process container for an application is currently running.
///
/// # Arguments
///
/// * `docker_client` - Docker API client ([`Docker`]).
/// * `app_id` - Target application ID string.
///
/// # Returns
///
/// A [`DockerResult`] containing a boolean flag.
pub async fn is_web_running(docker_client: &Docker, app_id: &str) -> DockerResult<bool> {
    let mut filters = HashMap::new();
    filters.insert(
        "label".to_string(),
        vec![
            format!("{}={}", LABEL_APP_ID, app_id),
            format!("{}=web", LABEL_PROCESS_TYPE),
        ],
    );
    filters.insert("status".to_string(), vec!["running".to_string()]);

    let containers = docker_client
        .list_containers(Some(ListContainersOptions {
            all: false,
            filters: Some(filters),
            ..Default::default()
        }))
        .await?;

    Ok(!containers.is_empty())
}

/// Stops all running process containers for a deployment.
///
/// # Arguments
///
/// * `docker_client` - Docker API client ([`Docker`]).
/// * `deployment` - Target deployment model ([`Deployment`]).
pub async fn stop_deployment_processes(
    docker_client: &Docker,
    deployment: &Deployment,
) -> DockerResult<()> {
    let processes = list_deployment_processes(docker_client, &deployment.id).await?;

    let stop_futures = processes.into_iter().map(|process| {
        let docker_client = docker_client.clone();
        async move { utils::stop_container(&docker_client, &process.name, Some(10)).await }
    });

    futures_util::future::try_join_all(stop_futures).await?;

    Ok(())
}

/// Restarts all process containers for a deployment and reconnects log streaming.
///
/// # Arguments
///
/// * `docker_client` - Docker API client ([`Docker`]).
/// * `runtime` - Runtime state handle ([`Runtime`]).
/// * `app` - Target application model ([`App`]).
/// * `deployment` - Target deployment model ([`Deployment`]).
pub async fn restart_deployment_processes(
    docker_client: &Docker,
    runtime: &Runtime,
    app: &App,
    deployment: &Deployment,
) -> DockerResult<()> {
    let processes = list_deployment_processes(docker_client, &deployment.id).await?;
    let log = runtime
        .log_bus
        .writer(ResourceKind::Deployment, &deployment.id)
        .app_id(&app.id);

    let restart_futures = processes.into_iter().map(|process| {
        let docker_client = docker_client.clone();
        let log = log.clone();
        async move {
            utils::restart_container(&docker_client, &process.name).await?;

            let log_prefix = match process.process_type {
                ProcessType::Web => LogPrefix::Web(process.instance_index),
                ProcessType::Worker => LogPrefix::Worker(process.instance_index),
                _ => LogPrefix::System,
            };

            stream_container_logs(docker_client, log.prefix(log_prefix), process.name);

            Ok::<(), DockerError>(())
        }
    });

    futures_util::future::try_join_all(restart_futures).await?;

    Ok(())
}

/// Force-removes all containers belonging to a deployment.
///
/// # Arguments
///
/// * `docker_client` - Docker API client ([`Docker`]).
/// * `deployment` - Target deployment model ([`Deployment`]).
pub async fn remove_deployment_processes(
    docker_client: &Docker,
    deployment: &Deployment,
) -> DockerResult<()> {
    let processes = list_deployment_processes(docker_client, &deployment.id).await?;

    let delete_futures = processes.into_iter().map(|process| {
        let docker_client = docker_client.clone();
        async move { utils::remove_container(&docker_client, &process.name).await }
    });

    futures_util::future::try_join_all(delete_futures).await?;

    Ok(())
}
