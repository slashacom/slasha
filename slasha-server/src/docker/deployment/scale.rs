use futures_util::{StreamExt, TryStreamExt};
use std::{collections::HashMap, sync::Arc};

use bollard::{Docker, query_parameters::RemoveContainerOptionsBuilder};
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
    DeploymentError, DeploymentResult, Rollback, logs::Log, naming::process_container_name,
};

pub async fn scale_deployment_process(
    docker_client: &Docker,
    db_pool: &DbPool,
    proxy_sync_trigger: &Arc<Notify>,
    log: &Log,
    app: &App,
    deployment: &Deployment,
    process_type: ProcessType,
    target_count: u32,
    scaling_lock: Arc<tokio::sync::Mutex<()>>,
) -> DeploymentResult<()> {
    let _guard = scaling_lock.lock().await;

    let mut rollback = Rollback::new();

    if let Err(e) = scale_deployment_process_inner(
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

async fn scale_deployment_process_inner(
    docker_client: &Docker,
    db_pool: &DbPool,
    proxy_sync_trigger: &Arc<Notify>,
    log: &Log,
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

    let processes = list_deployment_processes(docker_client, &deployment.id).await?;
    let existing_map: HashMap<u32, ProcessStatus> = processes
        .into_iter()
        .filter(|p| p.process_type == process_type)
        .map(|p| (p.instance_index, p.status))
        .collect();

    if target_count == existing_map.len() as u32 {
        return Ok(());
    }

    let deployment_context = resolve_deployment_context(db_pool, app, deployment).await?;
    let command = deployment_context
        .procfile
        .as_ref()
        .and_then(|pf| pf.commands.get(&process_type).cloned());

    let current_max_idx = existing_map.keys().copied().max();
    let end_idx = current_max_idx.unwrap_or(0).max(target_count - 1);

    log.send(format!(
        "Reconciling {} replicas to target count: {}",
        process_type, target_count
    ))
    .await?;

    let (comp_tx, mut comp_rx) = tokio::sync::mpsc::unbounded_channel();
    let deployment_context = Arc::new(deployment_context);
    let app = Arc::new(app.clone());
    let deployment = Arc::new(deployment.clone());
    let existing_map = Arc::new(existing_map);

    let reconciliation_result = futures_util::stream::iter(0..=end_idx)
        .map(Ok)
        .try_for_each_concurrent(None, |index| {
            let docker_client = docker_client.clone();
            let log = log.clone();
            let app = app.clone();
            let deployment = deployment.clone();
            let deployment_context = deployment_context.clone();
            let existing_map = existing_map.clone();
            let command = command.clone();
            let comp_tx = comp_tx.clone();

            async move {
                let process_status = existing_map.get(&index);

                if index < target_count {
                    if process_status.is_none() {
                        log.send(format!("Creating replica {}.{}", process_type, index))
                            .await?;

                        create_process_container(
                            &docker_client,
                            &app,
                            &deployment,
                            process_type,
                            index,
                            deployment_context.container_port,
                            command,
                            deployment_context.env_map.clone(),
                            deployment_context.volume_paths.clone(),
                        )
                        .await?;

                        let container_name = process_container_name(
                            &app.id,
                            &deployment.id,
                            &process_type.to_string().to_lowercase(),
                            index,
                        );

                        let dc = docker_client.clone();
                        let cn = container_name.clone();
                        let _ = comp_tx.send(Box::new(move || {
                            Box::pin(async move {
                                let _ = dc
                                    .remove_container(
                                        &cn,
                                        Some(
                                            RemoveContainerOptionsBuilder::new()
                                                .force(true)
                                                .build(),
                                        ),
                                    )
                                    .await;
                            })
                                as futures_util::future::BoxFuture<'static, ()>
                        }));

                        start_process_container(
                            &docker_client,
                            &log,
                            &app,
                            &deployment,
                            process_type,
                            index,
                        )
                        .await?;
                    } else if let Some(ProcessStatus::Stopped) = process_status {
                        log.send(format!("Restarting replica {}.{}", process_type, index))
                            .await?;

                        start_process_container(
                            &docker_client,
                            &log,
                            &app,
                            &deployment,
                            process_type,
                            index,
                        )
                        .await?;
                    }
                } else if process_status.is_some() {
                    log.send(format!(
                        "Removing excess replica {}.{}",
                        process_type, index
                    ))
                    .await?;

                    let container_name = process_container_name(
                        &app.id,
                        &deployment.id,
                        &process_type.to_string().to_lowercase(),
                        index,
                    );

                    let _ = docker_client
                        .stop_container(
                            &container_name,
                            Some(
                                bollard::query_parameters::StopContainerOptionsBuilder::new()
                                    .t(10)
                                    .build(),
                            ),
                        )
                        .await;

                    docker_client
                        .remove_container(
                            &container_name,
                            Some(RemoveContainerOptionsBuilder::new().force(true).build()),
                        )
                        .await?;
                }

                Ok::<(), DeploymentError>(())
            }
        })
        .await;

    while let Ok(comp) = comp_rx.try_recv() {
        rollback.register(comp);
    }

    reconciliation_result?;

    proxy_sync_trigger.notify_one();

    Ok(())
}
