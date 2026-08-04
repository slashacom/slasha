pub mod instance;
pub mod runner;

use bollard::Docker;
use runner::ServiceProvisionRunner;
use slasha_db::{
    app::App,
    logs::ResourceKind,
    repos::service::ServiceRepo,
    service::{Service, ServiceStatus},
};

pub use super::env::resolve_service_env;
use crate::{
    docker::{DockerResult, workflow::WorkflowRunner},
    state::AppState,
};

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
