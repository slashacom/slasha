use std::collections::HashMap;

use bollard::Docker;
use slasha_db::{app::App, deployment::Deployment, models::app_scale::ProcessType};

use crate::{
    docker::{
        DockerError, DockerResult,
        app::deploy::create::{CreateContainerContext, create_process_container},
        naming::process_container_name,
        utils::{self, stream_container_logs},
    },
    logs::LogWriter,
};

/// Removes a one-off container when dropped, unless disarmed first.
///
/// A cancelled deployment drops the release future mid-await, so the explicit
/// removal at the end never runs. The guard covers that path by spawning the
/// removal instead.
struct ContainerGuard {
    docker_client: Docker,
    name: String,
    armed: bool,
}

impl Drop for ContainerGuard {
    fn drop(&mut self) {
        if !self.armed {
            return;
        }

        let docker_client = self.docker_client.clone();
        let name = std::mem::take(&mut self.name);
        if let Ok(handle) = tokio::runtime::Handle::try_current() {
            handle.spawn(async move {
                let _ = utils::remove_container(&docker_client, &name).await;
            });
        }
    }
}

/// Runs an ephemeral release phase container and waits for completion.
///
/// The container is always removed afterwards, whether the command succeeded,
/// failed or the deployment was cancelled, and a container left over from an
/// earlier attempt of the same deployment is removed before this one is
/// created, since both share a name.
///
/// # Arguments
///
/// * `docker_client` - Docker API client ([`Docker`]).
/// * `log` - Log writer for output streaming ([`LogWriter`]).
/// * `app` - Target application model ([`App`]).
/// * `deployment` - Target deployment model ([`Deployment`]).
/// * `cmd` - Release command string.
/// * `env_map` - Resolved environment variable map.
pub async fn run_release_container(
    docker_client: &Docker,
    log: &LogWriter,
    app: &App,
    deployment: &Deployment,
    cmd: &str,
    env_map: &HashMap<String, String>,
) -> DockerResult<()> {
    log.stdout(format!("Running release command: {}", cmd));

    let release_container_name =
        process_container_name(&app.id, &deployment.id, &ProcessType::Release, 0);

    utils::remove_container(docker_client, &release_container_name).await?;

    let mut guard = ContainerGuard {
        docker_client: docker_client.clone(),
        name: release_container_name.clone(),
        armed: true,
    };

    let result = run_to_exit(
        docker_client,
        log,
        app,
        deployment,
        cmd,
        env_map,
        &release_container_name,
    )
    .await;

    guard.armed = false;
    match utils::remove_container(docker_client, &release_container_name).await {
        Ok(()) => tracing::info!(container = %release_container_name, "Container destroyed"),
        Err(e) => tracing::warn!(
            container = %release_container_name,
            error = ?e,
            "Failed to remove release container"
        ),
    }

    let exit_code = result?;
    if exit_code != 0 {
        return Err(DockerError::ReleaseFailed(exit_code));
    }

    log.stdout("Release command completed successfully");

    Ok(())
}

async fn run_to_exit(
    docker_client: &Docker,
    log: &LogWriter,
    app: &App,
    deployment: &Deployment,
    cmd: &str,
    env_map: &HashMap<String, String>,
    container_name: &str,
) -> DockerResult<i64> {
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

    utils::start_container(docker_client, container_name).await?;

    let stream_handle = stream_container_logs(
        docker_client.clone(),
        log.clone(),
        container_name.to_string(),
    );

    let exit_code = utils::wait_for_exit(docker_client, container_name).await;

    if let Ok(Err(e)) = stream_handle.await {
        tracing::warn!(container = %container_name, error = ?e, "Release log stream failed");
    }

    exit_code
}
