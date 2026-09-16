pub mod build;
pub mod context;
pub mod create;
pub mod readiness;
pub mod release;
pub mod runner;

use bollard::Docker;
use context::resolve_deployment_context;
use slasha_db::{
    app::App,
    deployment::{Deployment, DeploymentStatus},
    logs::ResourceKind,
    repos::deployment::DeploymentRepo,
};

use crate::{
    docker::{DockerResult, app::deploy::runner::DeploymentRunner, workflow::WorkflowRunner},
    state::AppState,
};

/// Runs the background deployment workflow for an application within a [`WorkflowRunner`].
///
/// # Arguments
///
/// * `state` - Application state holding database and runtime handles ([`AppState`]).
/// * `app` - Target application model ([`App`]).
/// * `docker_client` - Docker API client for the target node ([`Docker`]).
/// * `deployment` - Target deployment model ([`Deployment`]).
/// * `source_image` - Optional pre-existing image tag to reuse instead of building.
/// * `cancel_token` - Cancellation token to interrupt execution ([`CancellationToken`](tokio_util::sync::CancellationToken)).
///
/// # Returns
///
/// A [`DockerResult`] indicating whether the deployment workflow completed successfully.
pub async fn run_deployment_workflow(
    state: AppState,
    app: App,
    docker_client: Docker,
    deployment: Deployment,
    source_image: Option<String>,
    cancel_token: tokio_util::sync::CancellationToken,
) -> DockerResult<()> {
    tracing::info!(
        app_slug = %app.slug,
        deployment_id = %deployment.id,
        "deployment start"
    );

    let log_writer = state
        .runtime
        .log_bus
        .writer(ResourceKind::Deployment, &deployment.id)
        .app_id(&app.id);

    let result = WorkflowRunner::new(format!("deploy_app:{}", app.slug))
        .with_log(&log_writer)
        .with_cancel_token(&cancel_token)
        .run({
            let state = state.clone();
            let app = app.clone();
            let docker_client = docker_client.clone();
            let deployment = deployment.clone();
            let source_image = source_image.clone();
            let log_writer = log_writer.clone();

            move |wf| async move {
                let db_pool = &state.storage.db_pool;
                let context = resolve_deployment_context(db_pool, &app, &deployment).await?;

                let runner = DeploymentRunner {
                    state: &state,
                    app: &app,
                    docker_client: &docker_client,
                    deployment: &deployment,
                    wf: &wf,
                    log: &log_writer,
                    context: &context,
                };

                runner.execute(source_image.as_deref()).await
            }
        })
        .await;

    if let Err(error) = result {
        tracing::info!(
            app_slug = %app.slug,
            deployment_id = %deployment.id,
            status = "failed",
            error = ?error,
            "deployment finish"
        );

        log_writer.stdout(format!(
            "{error}\nRolling back this release; the previous deployment (if any) stays active"
        ));

        let _ = DeploymentRepo::update_status(
            &state.storage.db_pool,
            &deployment.id,
            DeploymentStatus::Failed,
        )
        .await;

        return Err(error);
    }

    tracing::info!(
        app_slug = %app.slug,
        deployment_id = %deployment.id,
        status = "success",
        "deployment finish"
    );

    Ok(())
}
