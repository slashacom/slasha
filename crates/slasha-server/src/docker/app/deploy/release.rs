use std::collections::HashMap;

use bollard::{Docker, query_parameters::WaitContainerOptions};
use futures_util::StreamExt;
use slasha_db::{app::App, deployment::Deployment, models::app_scale::ProcessType};

use crate::{
    docker::{
        DockerError, DockerResult,
        app::deploy::create::{CreateContainerContext, create_process_container},
        naming::process_container_name,
        utils,
    },
    logs::{LogHandle, stream_container_logs},
};

/// Runs an ephemeral release phase container and waits for completion.
///
/// # Arguments
///
/// * `docker_client` - Docker API client ([`Docker`]).
/// * `log` - Log handle for output streaming ([`LogHandle`]).
/// * `app` - Target application model ([`App`]).
/// * `deployment` - Target deployment model ([`Deployment`]).
/// * `cmd` - Release command string.
/// * `env_map` - Resolved environment variable map.
pub async fn run_release_container(
    docker_client: &Docker,
    log: &LogHandle,
    app: &App,
    deployment: &Deployment,
    cmd: &str,
    env_map: &HashMap<String, String>,
) -> DockerResult<()> {
    log.send(format!("Running release command: {}", cmd))
        .await?;

    create_process_container(
        docker_client,
        app,
        deployment,
        CreateContainerContext {
            process_type: ProcessType::Release,
            instance_index: 0,
            container_port: None,
            cmd: Some(cmd),
            env_map,
            volume_paths: &[],
            backup: None,
            litestream_volume: None,
        },
    )
    .await?;

    let release_container_name =
        process_container_name(&app.id, &deployment.id, &ProcessType::Release, 0);

    utils::start_container(docker_client, &release_container_name).await?;

    let stream_handle = stream_container_logs(
        docker_client.clone(),
        log.clone(),
        release_container_name.clone(),
        Some("[release]".to_string()),
    );

    if let Ok(deployment_result) = stream_handle.await {
        deployment_result?;
    }

    let wait_res = docker_client
        .wait_container(
            &release_container_name,
            Some(WaitContainerOptions {
                condition: "not-running".to_string(),
            }),
        )
        .next()
        .await
        .ok_or_else(|| {
            DockerError::BuildFailed("Release container wait stream ended prematurely".to_string())
        })??;

    let exit_code = wait_res.status_code;

    if let Err(e) = docker_client
        .remove_container(
            &release_container_name,
            Some(
                bollard::query_parameters::RemoveContainerOptionsBuilder::new()
                    .force(true)
                    .build(),
            ),
        )
        .await
    {
        tracing::warn!(
            container = %release_container_name,
            error = ?e,
            "Failed to remove release container"
        );
    } else {
        tracing::info!(container = %release_container_name, "Container destroyed");
    }

    if exit_code != 0 {
        return Err(DockerError::ReleaseFailed(exit_code));
    }

    log.send("Release command completed successfully").await?;

    Ok(())
}
