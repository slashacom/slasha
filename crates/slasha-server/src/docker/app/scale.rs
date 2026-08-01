use std::collections::HashMap;

use bollard::Docker;
use futures_util::future::try_join_all;
use slasha_db::{
    app::App,
    deployment::Deployment,
    models::app_scale::{NewAppScale, ProcessStatus, ProcessType},
    repos::{app_scale::AppScaleRepo, node::NodeRepo},
};

use super::{
    deploy::create::{CreateContainerContext, create_process_container},
    process::{list_deployment_processes, start_process_container},
};
use crate::{
    docker::{
        DockerError, DockerResult, app::deploy::context::resolve_deployment_context,
        naming::process_container_name, utils,
    },
    logs::LogKey,
    state::AppState,
};

/// Scales process replicas for a deployment up or down to match a target count.
///
/// # Arguments
///
/// * `state` - Application state holding database and runtime handles ([`AppState`]).
/// * `app` - Target application model ([`App`]).
/// * `deployment` - Target deployment model ([`Deployment`]).
/// * `process_type` - Process type enum ([`ProcessType`]).
/// * `target_count` - Target replica count.
pub async fn scale_deployment_process(
    state: &AppState,
    app: &App,
    deployment: &Deployment,
    process_type: ProcessType,
    target_count: u32,
) -> DockerResult<()> {
    let node = NodeRepo::get(&state.storage.db_pool, &app.node_id).await?;
    let docker_client = state.clients.docker_registry.get_client(&node)?;

    let log_key = LogKey::Deployment {
        app_slug: app.slug.clone(),
        deployment_id: deployment.id.clone(),
    };
    let log = state.runtime.log_manager.get_logger(&log_key).await?;

    if process_type == ProcessType::Release {
        return Err(DockerError::ScaleError(
            "Cannot scale release processes".to_string(),
        ));
    }

    if process_type == ProcessType::Web && target_count == 0 {
        return Err(DockerError::ScaleError(
            "Cannot scale web process to 0".to_string(),
        ));
    }

    AppScaleRepo::upsert(
        &state.storage.db_pool,
        NewAppScale {
            app_id: app.id.clone(),
            process_type,
            desired: target_count as i32,
        },
    )
    .await?;

    let existing = existing_process_replicas(&docker_client, &deployment.id, process_type).await?;

    let all_running =
        (0..target_count).all(|i| matches!(existing.get(&i), Some(ProcessStatus::Running)));
    let no_excess = existing.len() as u32 == target_count;

    if all_running && no_excess {
        return Ok(());
    }

    let deployment_ctx =
        resolve_deployment_context(&state.storage.db_pool, app, deployment).await?;
    let command = deployment_ctx
        .procfile
        .as_ref()
        .and_then(|pf| pf.get_process_command(process_type));

    log.send(format!(
        "Reconciling {} replicas to target count: {}",
        process_type, target_count
    ))
    .await?;

    for index in 0..target_count {
        match existing.get(&index) {
            None => {
                log.send(format!("Creating replica {}.{}", process_type, index))
                    .await?;

                create_process_container(
                    &docker_client,
                    app,
                    deployment,
                    CreateContainerContext {
                        process_type,
                        instance_index: index,
                        container_port: Some(deployment_ctx.container_port),
                        cmd: command,
                        env_map: &deployment_ctx.env_map,
                        volume_paths: &deployment_ctx.volume_paths,
                        backup: None,
                        litestream_volume: None,
                    },
                )
                .await?;

                start_process_container(&docker_client, &log, app, deployment, process_type, index)
                    .await?;
            }

            Some(ProcessStatus::Stopped) => {
                log.send(format!("Restarting replica {}.{}", process_type, index))
                    .await?;

                start_process_container(&docker_client, &log, app, deployment, process_type, index)
                    .await?;
            }

            Some(ProcessStatus::Running) => {}
        }
    }

    let remove_futures: Vec<_> = existing
        .keys()
        .copied()
        .filter(|&index| index >= target_count)
        .map(|index| {
            let docker_client = docker_client.clone();
            let name = process_container_name(&app.id, &deployment.id, &process_type, index);
            let log = log.clone();

            async move {
                log.send(format!(
                    "Removing excess replica {}.{}",
                    process_type, index
                ))
                .await?;

                if let Err(e) = utils::stop_container(&docker_client, &name, Some(10)).await {
                    tracing::warn!(container = %name, error = ?e, "Failed to stop excess replica container");
                }
                if let Err(e) = utils::remove_container(&docker_client, &name).await {
                    tracing::warn!(container = %name, error = ?e, "Failed to remove excess replica container");
                }

                Ok::<(), DockerError>(())
            }
        })
        .collect();

    try_join_all(remove_futures).await?;

    state.runtime.proxy_sync_trigger.notify_one();

    Ok(())
}

/// Inspects host Docker daemon to list existing container replicas for a specific process type.
///
/// # Arguments
///
/// * `docker_client` - Docker API client ([`Docker`]).
/// * `deployment_id` - Deployment ID string.
/// * `process_type` - Process type enum ([`ProcessType`]).
///
/// # Returns
///
/// A [`DockerResult`] containing a map of instance indices to [`ProcessStatus`].
async fn existing_process_replicas(
    docker_client: &Docker,
    deployment_id: &str,
    process_type: ProcessType,
) -> DockerResult<HashMap<u32, ProcessStatus>> {
    let processes = list_deployment_processes(docker_client, deployment_id).await?;

    Ok(processes
        .into_iter()
        .filter(|p| p.process_type == process_type)
        .map(|p| (p.instance_index, p.status))
        .collect())
}
