pub mod instance;
pub mod runner;

use bollard::Docker;
use runner::ServiceProvisionRunner;
use slasha_db::{
    app::App,
    logs::ResourceKind,
    repos::service::ServiceRepo,
    service::{Service, ServiceResources, ServiceStatus},
};

pub use super::env::resolve_service_env;
use crate::{
    docker::{DockerError, DockerResult, workflow::WorkflowRunner},
    state::AppState,
};

/// Validates requested service resource limits against host node capacity caps.
///
/// # Arguments
///
/// * `docker_client` - Docker API client ([`Docker`]).
/// * `resources` - Requested resource limits ([`ServiceResources`]).
///
/// # Returns
///
/// A [`DockerResult`] indicating whether the resource configuration is valid.
pub async fn validate_resources(
    docker_client: &Docker,
    resources: &ServiceResources,
) -> DockerResult<()> {
    const MIN_MEMORY_BYTES: i64 = 64 * 1024 * 1024;
    const MIN_NANO_CPUS: i64 = 100_000_000;
    const MIN_SHM_BYTES: i64 = 64 * 1024 * 1024;
    const MIN_PIDS_LIMIT: i64 = 64;

    if let Some(mem) = resources.memory_bytes
        && mem < MIN_MEMORY_BYTES
    {
        return Err(DockerError::Validation(format!(
            "memory must be at least {} MB",
            MIN_MEMORY_BYTES / (1024 * 1024)
        )));
    }
    if let Some(nc) = resources.nano_cpus
        && nc < MIN_NANO_CPUS
    {
        return Err(DockerError::Validation(
            "CPU must be at least 0.1 cores".into(),
        ));
    }
    if let Some(shm) = resources.shm_size
        && shm < MIN_SHM_BYTES
    {
        return Err(DockerError::Validation(format!(
            "shared memory must be at least {} MB",
            MIN_SHM_BYTES / (1024 * 1024)
        )));
    }
    if let Some(pids) = resources.pids_limit
        && pids < MIN_PIDS_LIMIT
    {
        return Err(DockerError::Validation(format!(
            "PID limit must be at least {}",
            MIN_PIDS_LIMIT
        )));
    }

    let info = docker_client.info().await?;

    if let Some(host_mem) = info.mem_total
        && let Some(mem) = resources.memory_bytes
    {
        let max_allowed_mem = (host_mem as f64 * 0.80) as i64;
        if mem > max_allowed_mem {
            return Err(DockerError::Validation(format!(
                "Requested memory ({} MB) exceeds 80% host capacity cap ({} MB of {} MB total host RAM)",
                mem / (1024 * 1024),
                max_allowed_mem / (1024 * 1024),
                host_mem / (1024 * 1024)
            )));
        }
    }
    if let Some(host_cpus) = info.ncpu
        && let Some(nc) = resources.nano_cpus
    {
        let host_nano = host_cpus.saturating_mul(1_000_000_000);
        if nc > host_nano {
            return Err(DockerError::Validation(format!(
                "CPU ({:.2} cores) exceeds host capacity ({} cores)",
                nc as f64 / 1_000_000_000.0,
                host_cpus
            )));
        }
    }
    if let Some(host_mem) = info.mem_total
        && let Some(shm) = resources.shm_size
        && shm > host_mem
    {
        return Err(DockerError::Validation(format!(
            "shared memory ({} MB) exceeds host capacity ({} MB)",
            shm / (1024 * 1024),
            host_mem / (1024 * 1024)
        )));
    }

    Ok(())
}

/// Spawns background provisioning or redeployment of a database service container inside a [`WorkflowRunner`].
///
/// # Arguments
///
/// * `state` - Application state holding database and runtime handles ([`AppState`]).
/// * `app` - Target application model ([`App`]).
/// * `docker_client` - Docker API client ([`Docker`]).
/// * `service` - Target database service model ([`Service`]).
/// * `is_redeploy` - Whether this workflow is redeploying an existing service.
///
/// # Returns
///
/// A [`DockerResult`] indicating whether the provisioning workflow succeeded.
pub async fn run_provision_service_workflow(
    state: AppState,
    app: App,
    docker_client: Docker,
    service: Service,
    is_redeploy: bool,
) -> DockerResult<()> {
    tracing::info!(
        app_slug = %app.slug,
        service_id = %service.id,
        service_name = %service.name,
        "service provision start"
    );

    let log_writer = state
        .runtime
        .log_bus
        .writer(ResourceKind::Service, &service.id)
        .app_id(&app.id);

    let result = WorkflowRunner::new(format!("provision_service:{}", service.name))
        .with_log(&log_writer)
        .run({
            let state = state.clone();
            let app = app.clone();
            let docker_client = docker_client.clone();
            let service = service.clone();
            let log_writer = log_writer.clone();

            move |wf| async move {
                let runner = ServiceProvisionRunner {
                    state: &state,
                    app: &app,
                    service: &service,
                    docker_client: &docker_client,
                    wf: &wf,
                    log: &log_writer,
                };

                runner.execute(is_redeploy).await
            }
        })
        .await;

    if let Err(e) = result {
        tracing::error!(
            app_id = %service.app_id,
            service_id = %service.id,
            service_name = %service.name,
            error = ?e,
            "service provision failed"
        );

        log_writer.stdout(format!("Service provision failed: {}", e));
        let _ =
            ServiceRepo::update_status(&state.storage.db_pool, &service.id, ServiceStatus::Failed)
                .await;

        state.runtime.log_bus.remove(&service.id);
        return Err(e);
    }

    tracing::info!(
        app_slug = %app.slug,
        service_id = %service.id,
        service_name = %service.name,
        "service provision finish"
    );

    Ok(())
}
