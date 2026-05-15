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
    deployment::{Deployment, DeploymentStatus},
    models::app_scale::{ProcessContainer, ProcessStatus, ProcessType},
    repos::deployment::DeploymentRepo,
};
use tokio::sync::Notify;

use crate::{
    docker::{
        DeploymentError, DeploymentResult, image_tag,
        log_driver::default_log_config,
        logs::{Log, LogKey, LogManager, stream_container_logs},
        naming::{
            app_network_name, app_volume_name, app_volume_prefix, process_container_name,
            release_container_name,
        },
    },
    proxy::container::PROXY_NETWORK_NAME,
};

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

pub async fn create_process_container(
    docker_client: &Docker,
    app: &App,
    deployment: &Deployment,
    process_type: ProcessType,
    instance_index: u32,
    container_port: u16,
    cmd: Option<String>,
    env_map: HashMap<String, String>,
    volume_paths: Vec<String>,
) -> DeploymentResult<()> {
    let container_name = process_container_name(
        &app.id,
        &deployment.id,
        &process_type.to_string().to_lowercase(),
        instance_index,
    );

    let mounts = build_mounts(docker_client, &app.id, &volume_paths).await?;

    let mut labels: HashMap<String, String> = HashMap::new();
    labels.insert("slasha.managed".into(), "true".into());
    labels.insert("slasha.app_id".into(), app.id.clone());
    labels.insert("slasha.deployment_id".into(), deployment.id.clone());
    labels.insert("slasha.app_slug".into(), app.slug.clone());
    if process_type == ProcessType::Web {
        labels.insert("slasha.container_port".into(), container_port.to_string());
    }
    labels.insert("slasha.process_type".into(), process_type.to_string());
    labels.insert("slasha.instance_index".into(), instance_index.to_string());

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
                name: Some(container_name),
                ..Default::default()
            }),
            ContainerCreateBody {
                image: Some(image_tag(&app.slug, &deployment.commit_sha)),
                labels: Some(labels),
                env,
                cmd: cmd.map(|c| vec!["sh".to_string(), "-c".to_string(), c]),
                host_config: Some(HostConfig {
                    restart_policy: Some(match process_type {
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

    Ok(())
}

pub async fn start_deployment_processes(
    docker_client: &Docker,
    db_pool: &DbPool,
    proxy_sync_trigger: &Arc<Notify>,
    log: &Log,
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
    log: &Log,
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

    for process in processes {
        if let Err(e) = docker_client
            .stop_container(
                &process.name,
                Some(StopContainerOptionsBuilder::new().t(10).build()),
            )
            .await
        {
            tracing::warn!("Failed to stop container {}: {:?}", process.name, e);
        }
    }

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

        tokio::spawn({
            let docker = docker_client.clone();
            let log = log.clone();
            let container_name = process.name.clone();
            async move {
                if let Err(e) =
                    stream_container_logs(docker, log, container_name, Some(prefix)).await
                {
                    tracing::error!("Failed to stream deployment logs: {}", e);
                }
            }
        });
    }

    proxy_sync_trigger.notify_one();

    Ok(())
}

pub async fn delete_deployment_processes(
    docker_client: &Docker,
    proxy_sync_trigger: &Arc<Notify>,
    log_manager: &LogManager,
    app: &App,
    deployment: &Deployment,
) -> DeploymentResult<()> {
    let processes = list_deployment_processes(docker_client, &deployment.id).await?;

    for process in processes {
        if let Err(e) = docker_client
            .remove_container(
                &process.name,
                Some(RemoveContainerOptionsBuilder::new().force(true).build()),
            )
            .await
        {
            tracing::warn!("Failed to remove container {}: {:?}", process.name, e);
        }
    }

    log_manager.remove(&LogKey::Deployment {
        app_slug: app.slug.clone(),
        deployment_id: deployment.id.clone(),
    });

    proxy_sync_trigger.notify_one();

    Ok(())
}

pub async fn delete_app_volumes(docker_client: &Docker, app_id: &str) -> DeploymentResult<()> {
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
            tracing::warn!("Failed to remove volume {}: {:?}", name, e);
        }
    }

    Ok(())
}

async fn start_and_stream(
    docker_client: &Docker,
    log: &Log,
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

    tokio::spawn({
        let docker_client = docker_client.clone();
        let log = log.clone();
        async move {
            if let Err(e) = stream_container_logs(docker_client, log, container_name, prefix).await
            {
                tracing::warn!("Log stream ended with error: {:?}", e);
            }
        }
    });

    Ok(())
}

async fn build_mounts(
    docker_client: &Docker,
    app_id: &str,
    volume_paths: &[String],
) -> DeploymentResult<Vec<Mount>> {
    let mut mounts = Vec::with_capacity(volume_paths.len());

    for path in volume_paths {
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
    log: &Log,
    app: &App,
    deployment: &Deployment,
    cmd: String,
    env_map: HashMap<String, String>,
) -> DeploymentResult<()> {
    let container_name = release_container_name(&app.id, &deployment.id);

    log.send(format!("Running release command: {}", cmd))
        .await?;

    create_process_container(
        docker_client,
        app,
        deployment,
        ProcessType::Release,
        0,
        0,
        Some(cmd),
        env_map,
        Vec::new(),
    )
    .await?;

    docker_client
        .start_container(
            &container_name,
            Some(StartContainerOptionsBuilder::new().build()),
        )
        .await?;

    stream_container_logs(
        docker_client.clone(),
        log.clone(),
        container_name.clone(),
        Some("[release]".to_string()),
    )
    .await?;

    let wait_res = docker_client
        .wait_container(
            &container_name,
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
            &container_name,
            Some(RemoveContainerOptionsBuilder::new().force(true).build()),
        )
        .await
    {
        tracing::warn!(
            "Failed to remove release container {}: {:?}",
            container_name,
            e
        );
    }

    if exit_code != 0 {
        return Err(DeploymentError::ReleaseFailed(exit_code));
    }

    log.send("Release command completed successfully").await?;

    Ok(())
}
