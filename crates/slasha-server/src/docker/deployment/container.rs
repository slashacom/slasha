use std::{collections::HashMap, sync::Arc};

use bollard::{
    Docker,
    models::{
        ContainerCreateBody, EndpointSettings, HostConfig, Mount, MountTypeEnum, NetworkingConfig,
        RestartPolicy, RestartPolicyNameEnum, VolumeCreateRequest,
    },
    plugin::ContainerSummaryStateEnum,
    query_parameters::{
        CreateContainerOptions, ListContainersOptions, ListVolumesOptions,
        RemoveContainerOptionsBuilder, RemoveVolumeOptions, StartContainerOptionsBuilder,
        StopContainerOptionsBuilder, WaitContainerOptions,
    },
};
use futures_util::StreamExt;
use slasha_db::{
    DbPool,
    app::App,
    app_backup::AppBackup,
    deployment::{Deployment, DeploymentStatus},
    models::app_scale::{ProcessContainer, ProcessStatus, ProcessType},
    repos::deployment::DeploymentRepo,
};
use tokio::sync::Notify;

use crate::{
    docker::{
        DeploymentError, DeploymentResult,
        deployment::litestream,
        image_tag,
        log_driver::default_log_config,
        logs::{LogHandle, LogKey, LogManager, stream_container_logs},
        naming::{
            app_network_name, app_volume_name, app_volume_prefix, process_container_name,
            release_container_name,
        },
    },
    proxy::container::PROXY_NETWORK_NAME,
};

// per-app persistent volume mount path, mounted into every process container
// exposed to the app as`SLASHA_DATA_DIR`
pub const MANAGED_DATA_PATH: &str = "/data";

pub async fn list_deployment_processes(
    docker_client: &Docker,
    deployment_id: &str,
) -> DeploymentResult<Vec<ProcessContainer>> {
    let mut filters = HashMap::new();
    filters.insert(
        "label".to_string(),
        vec![format!("slasha.deployment_id={}", deployment_id)],
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
                .get("slasha.process_type")
                .and_then(|s| std::str::FromStr::from_str(s).ok())?;
            let instance_index = labels
                .get("slasha.instance_index")
                .and_then(|s| s.parse::<u32>().ok())?;

            let status = match c.state {
                Some(ContainerSummaryStateEnum::RUNNING) => ProcessStatus::Running,
                _ => ProcessStatus::Stopped,
            };

            Some(ProcessContainer {
                name,
                process_type,
                instance_index,
                status,
            })
        })
        .collect();

    Ok(processes)
}

pub async fn is_web_running(docker_client: &Docker, app_id: &str) -> DeploymentResult<bool> {
    let mut filters = HashMap::new();
    filters.insert(
        "label".to_string(),
        vec![
            format!("slasha.app_id={}", app_id),
            "slasha.process_type=web".to_string(),
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

pub struct CreateContainerContext {
    pub process_type: ProcessType,
    pub instance_index: u32,
    pub container_port: Option<u16>,
    pub cmd: Option<String>,
    pub env_map: HashMap<String, String>,
    pub volume_paths: Vec<String>,
    pub backup: Option<AppBackup>,
    pub litestream_volume: Option<String>,
}

pub async fn create_process_container(
    docker_client: &Docker,
    app: &App,
    deployment: &Deployment,
    context: CreateContainerContext,
) -> DeploymentResult<()> {
    let container_name = process_container_name(
        &app.id,
        &deployment.id,
        &context.process_type.to_string().to_lowercase(),
        context.instance_index,
    );

    let mut mounts = build_mounts(docker_client, &app.id, &context.volume_paths).await?;

    let mut cmd = context.cmd;
    let mut env_map = context.env_map;

    // wrap the primary web instance with Litestream so its SQLite database is
    // restored on boot and continuously replicated. Litestream must be a single
    // writer, so only web instance 0 is ever wrapped.
    if let Some(backup) = &context.backup
        && backup.enabled
        && context.process_type == ProcessType::Web
        && context.instance_index == 0
    {
        match (&cmd, &context.litestream_volume) {
            (Some(original_cmd), Some(_volume)) => {
                let plan = litestream::plan(backup, original_cmd, backup.restore_pending);
                cmd = Some(plan.command);
                env_map.extend(plan.env);
                mounts.push(litestream::binary_mount());
            }
            (None, _) => tracing::warn!(
                app_id = %app.id,
                "backups enabled but the web process has no start command; skipping replication"
            ),
            (_, None) => tracing::warn!(
                app_id = %app.id,
                "backups enabled but the litestream binary is unavailable; skipping replication"
            ),
        }
    }

    let mut labels: HashMap<String, String> = HashMap::new();
    labels.insert("slasha.managed".into(), "true".into());
    labels.insert("slasha.app_id".into(), app.id.clone());
    labels.insert("slasha.deployment_id".into(), deployment.id.clone());
    labels.insert("slasha.app_slug".into(), app.slug.clone());
    if let Some(container_port) = context.container_port
        && context.process_type == ProcessType::Web
    {
        labels.insert("slasha.container_port".into(), container_port.to_string());
    }
    labels.insert(
        "slasha.process_type".into(),
        context.process_type.to_string(),
    );
    labels.insert(
        "slasha.instance_index".into(),
        context.instance_index.to_string(),
    );

    let env: Option<Vec<String>> = if env_map.is_empty() {
        None
    } else {
        Some(
            env_map
                .into_iter()
                .map(|(k, v)| format!("{}={}", k, v))
                .collect(),
        )
    };

    let app_network = app_network_name(&app.id);
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

    docker_client
        .create_container(
            Some(CreateContainerOptions {
                name: Some(container_name.clone()),
                ..Default::default()
            }),
            ContainerCreateBody {
                image: Some(image_tag(&app.slug, &deployment.commit_sha)),
                labels: Some(labels),
                env,
                cmd: cmd.map(|c| vec!["sh".to_string(), "-c".to_string(), c]),
                host_config: Some(HostConfig {
                    restart_policy: Some(match context.process_type {
                        ProcessType::Release => RestartPolicy {
                            name: Some(RestartPolicyNameEnum::EMPTY),
                            maximum_retry_count: None,
                        },
                        _ => RestartPolicy {
                            name: Some(RestartPolicyNameEnum::UNLESS_STOPPED),
                            maximum_retry_count: None,
                        },
                    }),
                    mounts: if mounts.is_empty() {
                        None
                    } else {
                        Some(mounts)
                    },
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

    tracing::info!(
        container = %container_name,
        app_id = %app.id,
        deployment_id = %deployment.id,
        process_type = %context.process_type,
        "container created"
    );

    Ok(())
}

pub async fn start_deployment_processes(
    docker_client: &Docker,
    db_pool: &DbPool,
    proxy_sync_trigger: &Arc<Notify>,
    log: &LogHandle,
    deployment_id: &str,
) -> DeploymentResult<()> {
    let processes = list_deployment_processes(docker_client, deployment_id).await?;

    for process in processes {
        let prefix = format!(
            "[{}.{}]",
            process.process_type.to_string().to_lowercase(),
            process.instance_index
        );
        start_and_stream(docker_client, log, process.name, Some(prefix)).await?;
    }

    DeploymentRepo::update_status(db_pool, deployment_id, DeploymentStatus::Running).await?;
    proxy_sync_trigger.notify_one();

    Ok(())
}

pub async fn start_process_container(
    docker_client: &Docker,
    log: &LogHandle,
    app: &App,
    deployment: &Deployment,
    process_type: ProcessType,
    instance_index: u32,
) -> DeploymentResult<()> {
    let container_name = process_container_name(
        &app.id,
        &deployment.id,
        &process_type.to_string().to_lowercase(),
        instance_index,
    );

    let prefix = format!(
        "[{}.{}]",
        process_type.to_string().to_lowercase(),
        instance_index
    );
    start_and_stream(docker_client, log, container_name, Some(prefix)).await
}

pub async fn stop_deployment_processes(
    docker_client: &Docker,
    db_pool: &DbPool,
    proxy_sync_trigger: &Arc<Notify>,
    log_manager: &LogManager,
    app: &App,
    deployment: &Deployment,
) -> DeploymentResult<()> {
    let processes = list_deployment_processes(docker_client, &deployment.id).await?;

    let stop_futures = processes.into_iter().map(|process| {
        let docker = docker_client.clone();
        async move {
            if let Err(e) = docker
                .stop_container(
                    &process.name,
                    Some(StopContainerOptionsBuilder::new().t(10).build()),
                )
                .await
            {
                tracing::warn!(
                    container = %process.name,
                    error = ?e,
                    "Failed to stop container"
                );
            }
        }
    });

    futures_util::future::join_all(stop_futures).await;

    log_manager.remove(&LogKey::Deployment {
        app_slug: app.slug.clone(),
        deployment_id: deployment.id.clone(),
    });

    DeploymentRepo::update_status(db_pool, &deployment.id, DeploymentStatus::Stopped).await?;
    proxy_sync_trigger.notify_one();

    Ok(())
}

pub async fn restart_deployment_processes(
    docker_client: &Docker,
    log_manager: &LogManager,
    proxy_sync_trigger: &Arc<Notify>,
    app: &App,
    deployment_id: &str,
) -> DeploymentResult<()> {
    let processes = list_deployment_processes(docker_client, deployment_id).await?;
    let log_key = LogKey::Deployment {
        app_slug: app.slug.clone(),
        deployment_id: deployment_id.to_string(),
    };
    let log = log_manager.get_logger(&log_key).await?;

    for process in processes {
        docker_client.restart_container(&process.name, None).await?;

        let prefix = format!(
            "[{}.{}]",
            process.process_type.to_string().to_lowercase(),
            process.instance_index
        );

        stream_container_logs(
            docker_client.clone(),
            log.clone(),
            process.name.clone(),
            Some(prefix),
        );
    }

    proxy_sync_trigger.notify_one();

    Ok(())
}

pub async fn remove_deployment_processes(
    docker_client: &Docker,
    proxy_sync_trigger: &Arc<Notify>,
    log_manager: &LogManager,
    app: &App,
    deployment: &Deployment,
) -> DeploymentResult<()> {
    let processes = list_deployment_processes(docker_client, &deployment.id).await?;

    let delete_futures = processes.into_iter().map(|process| {
        let docker_client = docker_client.clone();
        async move {
            if let Err(e) = docker_client
                .remove_container(
                    &process.name,
                    Some(RemoveContainerOptionsBuilder::new().force(true).build()),
                )
                .await
            {
                tracing::warn!(
                    container = %process.name,
                    error = ?e,
                    "Failed to remove container"
                );
            } else {
                tracing::info!(
                    container = %process.name,
                    "Container destroyed"
                );
            }
        }
    });

    futures_util::future::join_all(delete_futures).await;

    log_manager.remove(&LogKey::Deployment {
        app_slug: app.slug.clone(),
        deployment_id: deployment.id.clone(),
    });

    proxy_sync_trigger.notify_one();

    Ok(())
}

pub async fn remove_app_volumes(docker_client: &Docker, app_id: &str) -> DeploymentResult<()> {
    let prefix = app_volume_prefix(app_id);

    let mut filters: HashMap<String, Vec<String>> = HashMap::new();
    filters.insert("name".to_string(), vec![prefix.clone()]);

    let response = docker_client
        .list_volumes(Some(ListVolumesOptions {
            filters: Some(filters),
        }))
        .await?;

    let names: Vec<String> = response
        .volumes
        .unwrap_or_default()
        .into_iter()
        .map(|v| v.name)
        .filter(|n| n.starts_with(&prefix))
        .collect();

    for name in names {
        if let Err(e) = docker_client
            .remove_volume(&name, None::<RemoveVolumeOptions>)
            .await
        {
            tracing::warn!(
                volume = %name,
                error = ?e,
                "Failed to remove volume"
            );
        }
    }

    Ok(())
}

async fn start_and_stream(
    docker_client: &Docker,
    log: &LogHandle,
    container_name: String,
    prefix: Option<String>,
) -> DeploymentResult<()> {
    docker_client
        .start_container(
            &container_name,
            Some(StartContainerOptionsBuilder::new().build()),
        )
        .await?;

    log.send(format!("Container {} started", container_name))
        .await?;

    stream_container_logs(docker_client.clone(), log.clone(), container_name, prefix);

    Ok(())
}

async fn build_mounts(
    docker_client: &Docker,
    app_id: &str,
    volume_paths: &[String],
) -> DeploymentResult<Vec<Mount>> {
    let mut paths: Vec<String> = vec![MANAGED_DATA_PATH.to_string()];
    for path in volume_paths {
        // prevent multiple mounts of the same volume
        if path != MANAGED_DATA_PATH {
            paths.push(path.clone());
        }
    }

    let mut mounts = Vec::with_capacity(paths.len());

    for path in &paths {
        let volume_name = app_volume_name(app_id, path);
        docker_client
            .create_volume(VolumeCreateRequest {
                name: Some(volume_name.clone()),
                ..Default::default()
            })
            .await?;

        mounts.push(Mount {
            typ: Some(MountTypeEnum::VOLUME),
            source: Some(volume_name),
            target: Some(path.clone()),
            ..Default::default()
        });
    }

    Ok(mounts)
}

pub async fn run_release_container(
    docker_client: &Docker,
    log: &LogHandle,
    app: &App,
    deployment: &Deployment,
    cmd: String,
    env_map: HashMap<String, String>,
) -> DeploymentResult<()> {
    let release_container_name = release_container_name(&app.id, &deployment.id);

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
            volume_paths: Vec::new(),
            backup: None,
            litestream_volume: None,
        },
    )
    .await?;

    docker_client
        .start_container(
            &release_container_name,
            Some(StartContainerOptionsBuilder::new().build()),
        )
        .await?;

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
            DeploymentError::BuildFailed(
                "Release container wait stream ended prematurely".to_string(),
            )
        })??;

    let exit_code = wait_res.status_code;

    if let Err(e) = docker_client
        .remove_container(
            &release_container_name,
            Some(RemoveContainerOptionsBuilder::new().force(true).build()),
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
        return Err(DeploymentError::ReleaseFailed(exit_code));
    }

    log.send("Release command completed successfully").await?;

    Ok(())
}
