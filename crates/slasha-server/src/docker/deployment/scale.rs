use std::{collections::HashMap, sync::Arc};

use bollard::{
    Docker,
    query_parameters::{RemoveContainerOptionsBuilder, StopContainerOptionsBuilder},
};
use futures_util::future::try_join_all;
use slasha_db::{
    DbPool,
    app::App,
    deployment::Deployment,
    models::app_scale::{ProcessStatus, ProcessType},
    repos::app_scale::AppScaleRepo,
};
use tokio::sync::Notify;

use super::{
    container::{create_process_container, list_deployment_processes, start_process_container},
    executor::resolve_deployment_context,
};
use crate::docker::{
    DeploymentError, DeploymentResult, Rollback, logs::LogHandle, naming::process_container_name,
};

pub async fn scale_deployment_process(
    docker_client: &Docker,
    db_pool: &DbPool,
    proxy_sync_trigger: &Arc<Notify>,
    log: &LogHandle,
    app: &App,
    deployment: &Deployment,
    process_type: ProcessType,
    target_count: u32,
    scaling_lock: Arc<tokio::sync::Mutex<()>>,
) -> DeploymentResult<()> {
    let _guard = scaling_lock.lock().await;
    let mut rollback = Rollback::new();

    if let Err(e) = scale_inner(
        docker_client,
        db_pool,
        proxy_sync_trigger,
        log,
        app,
        deployment,
        process_type,
        target_count,
        &mut rollback,
    )
    .await
    {
        log.send(format!("Scaling failed: {}", e)).await?;
        rollback.execute().await;
        return Err(e);
    }

    rollback.disarm();
    AppScaleRepo::upsert(db_pool, &app.id, process_type, target_count as i32).await?;

    Ok(())
}

async fn scale_inner(
    docker_client: &Docker,
    db_pool: &DbPool,
    proxy_sync_trigger: &Arc<Notify>,
    log: &LogHandle,
    app: &App,
    deployment: &Deployment,
    process_type: ProcessType,
    target_count: u32,
    rollback: &mut Rollback,
) -> DeploymentResult<()> {
    if process_type == ProcessType::Release {
        return Err(DeploymentError::ScaleError(
            "Cannot scale release processes".to_string(),
        ));
    }

    if target_count == 0 {
        return Err(DeploymentError::ScaleError("Cannot scale to 0".to_string()));
    }

    let existing = existing_processes(docker_client, &deployment.id, process_type).await?;

    if target_count == existing.len() as u32 {
        return Ok(());
    }

    let context = resolve_deployment_context(db_pool, app, deployment).await?;
    let command = context
        .procfile
        .as_ref()
        .and_then(|pf| pf.commands.get(&process_type).cloned());

    log.send(format!(
        "Reconciling {} replicas to target count: {}",
        process_type, target_count
    ))
    .await?;

    let max_idx = existing
        .keys()
        .copied()
        .max()
        .unwrap_or(0)
        .max(target_count - 1);

    for index in 0..target_count {
        match existing.get(&index) {
            None => {
                log.send(format!("Creating replica {}.{}", process_type, index))
                    .await?;

                create_process_container(
                    docker_client,
                    app,
                    deployment,
                    process_type,
                    index,
                    Some(context.container_port),
                    command.clone(),
                    context.env_map.clone(),
                    context.volume_paths.clone(),
                )
                .await?;

                let container_name = process_container_name(
                    &app.id,
                    &deployment.id,
                    &process_type.to_string().to_lowercase(),
                    index,
                );

                rollback.register({
                    let docker = docker_client.clone();
                    let name = container_name.clone();
                    move || {
                        Box::pin(async move {
                            if let Err(e) = docker
                                .remove_container(
                                    &name,
                                    Some(RemoveContainerOptionsBuilder::new().force(true).build()),
                                )
                                .await
                            {
                                tracing::warn!(
                                    container = %name,
                                    error = ?e,
                                    "Failed to remove container during rollback"
                                );
                            }
                        })
                    }
                });

                start_process_container(docker_client, log, app, deployment, process_type, index)
                    .await?;
            }

            Some(ProcessStatus::Stopped) => {
                log.send(format!("Restarting replica {}.{}", process_type, index))
                    .await?;

                start_process_container(docker_client, log, app, deployment, process_type, index)
                    .await?;
            }

            Some(ProcessStatus::Running) => {}
        }
    }

    let remove_futures: Vec<_> = ((target_count)..=max_idx)
        .filter(|index| existing.contains_key(index))
        .map(|index| {
            let docker_client = docker_client.clone();
            let name = process_container_name(
                &app.id,
                &deployment.id,
                &process_type.to_string().to_lowercase(),
                index,
            );
            let log = log.clone();

            async move {
                log.send(format!(
                    "Removing excess replica {}.{}",
                    process_type, index
                ))
                .await?;

                if let Err(e) = docker_client
                    .stop_container(
                        &name,
                        Some(StopContainerOptionsBuilder::new().t(10).build()),
                    )
                    .await
                {
                    tracing::warn!(container = %name, error = ?e, "Failed to stop container");
                }

                if let Err(e) = docker_client
                    .remove_container(
                        &name,
                        Some(RemoveContainerOptionsBuilder::new().force(true).build()),
                    )
                    .await
                {
                    tracing::warn!(container = %name, error = ?e, "Failed to remove container");
                }

                Ok::<(), DeploymentError>(())
            }
        })
        .collect();

    try_join_all(remove_futures).await?;

    proxy_sync_trigger.notify_one();

    Ok(())
}

async fn existing_processes(
    docker_client: &Docker,
    deployment_id: &str,
    process_type: ProcessType,
) -> DeploymentResult<HashMap<u32, ProcessStatus>> {
    let processes = list_deployment_processes(docker_client, deployment_id).await?;

    Ok(processes
        .into_iter()
        .filter(|p| p.process_type == process_type)
        .map(|p| (p.instance_index, p.status))
        .collect())
}
