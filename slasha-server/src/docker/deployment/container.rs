use std::{collections::HashMap, sync::Arc};

use bollard::{
    Docker,
    models::{
        ContainerCreateBody, EndpointSettings, HostConfig, Mount, MountTypeEnum, NetworkingConfig,
        PortBinding, RestartPolicy, RestartPolicyNameEnum, VolumeCreateRequest,
    },
    query_parameters::{
        CreateContainerOptions, ListVolumesOptions, RemoveContainerOptionsBuilder,
        RemoveVolumeOptions, StartContainerOptionsBuilder, StopContainerOptionsBuilder,
    },
};
use slasha_db::{
    DbPool,
    app::App,
    deployment::{Deployment, DeploymentStatus},
    repos::deployment::DeploymentRepo,
};
use tokio::sync::Notify;

use crate::docker::{
    DeploymentError, DeploymentResult,
    logs::{Log, LogKey, LogManager, stream_container_logs},
    naming::{app_container_name, app_network_name, app_volume_name, app_volume_prefix},
    port_pool::PortPool,
};

async fn get_container_host_port(docker_client: &Docker, name: &str) -> DeploymentResult<u16> {
    let info = docker_client.inspect_container(name, None).await?;

    info.network_settings
        .and_then(|ns| ns.ports)
        .and_then(|ports| {
            ports
                .into_values()
                .flatten()
                .flatten()
                .next()
                .and_then(|pb| pb.host_port.and_then(|s| s.parse::<u16>().ok()))
        })
        .ok_or_else(|| {
            DeploymentError::PortAllocationFailed("No host port found in container inspect".into())
        })
}

pub async fn create_deployment_container(
    docker_client: &Docker,
    port_pool: &Arc<PortPool>,
    app: &App,
    deployment: &Deployment,
    container_port: u16,
    env_map: HashMap<String, String>,
    volume_paths: Vec<String>,
) -> DeploymentResult<(String, u16)> {
    let deployment_id = deployment.id.clone();
    let host_port = port_pool.allocate().await?;
    let name = app_container_name(&app.id, &deployment_id);
    let image = crate::docker::naming::image_tag(&app.slug, &deployment.commit_sha);

    let port_key = format!("{}/tcp", container_port);
    let mut port_bindings: HashMap<String, Option<Vec<PortBinding>>> = HashMap::new();
    port_bindings.insert(
        port_key,
        Some(vec![PortBinding {
            host_ip: Some("127.0.0.1".to_string()),
            host_port: Some(host_port.to_string()),
        }]),
    );

    let mut labels: HashMap<String, String> = HashMap::new();
    labels.insert("slasha.managed".into(), "true".into());
    labels.insert("slasha.app_id".into(), app.id.clone());
    labels.insert("slasha.deployment_id".into(), deployment_id.clone());
    labels.insert("slasha.app_slug".into(), app.slug.clone());
    labels.insert("slasha.host_port".into(), host_port.to_string());

    let mut mounts: Vec<Mount> = Vec::with_capacity(volume_paths.len());
    for path in &volume_paths {
        let volume_name = app_volume_name(&app.id, path);
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

    let host_config = HostConfig {
        port_bindings: Some(port_bindings),
        restart_policy: Some(RestartPolicy {
            name: Some(RestartPolicyNameEnum::UNLESS_STOPPED),
            maximum_retry_count: None,
        }),
        mounts: if mounts.is_empty() {
            None
        } else {
            Some(mounts)
        },
        ..Default::default()
    };

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

    let network_name = app_network_name(&app.id);
    let mut endpoints_config = HashMap::new();
    endpoints_config.insert(
        network_name.clone(),
        EndpointSettings {
            network_id: Some(network_name),
            ..Default::default()
        },
    );

    let container_config = ContainerCreateBody {
        image: Some(image),
        labels: Some(labels),
        host_config: Some(host_config),
        networking_config: Some(NetworkingConfig {
            endpoints_config: Some(endpoints_config),
        }),
        env,
        ..Default::default()
    };

    let create_opts = CreateContainerOptions {
        name: Some(name.clone()),
        ..Default::default()
    };

    docker_client
        .create_container(Some(create_opts), container_config)
        .await?;

    Ok((name, host_port))
}

pub async fn start_deployment_container(
    docker_client: &Docker,
    db_pool: &DbPool,
    proxy_reconcile: &Arc<Notify>,
    log: &Log,
    deployment_id: &str,
    container_name: &str,
    host_port: u16,
) -> DeploymentResult<()> {
    docker_client
        .start_container(
            container_name,
            Some(StartContainerOptionsBuilder::new().build()),
        )
        .await?;

    DeploymentRepo::update_status(db_pool, deployment_id, DeploymentStatus::Running).await?;

    proxy_reconcile.notify_one();

    log.send(format!(
        "Container {} started on host port {}",
        container_name, host_port
    ))
    .await?;

    tokio::spawn({
        let docker_client = docker_client.clone();
        let log = log.clone();
        let container_name = container_name.to_string();

        async move {
            if let Err(e) = stream_container_logs(docker_client, log, container_name).await {
                tracing::warn!("log stream ended with error: {:?}", e);
            }
        }
    });

    Ok(())
}

pub async fn stop_deployment_container(
    docker_client: &Docker,
    db_pool: &DbPool,
    port_pool: &Arc<PortPool>,
    proxy_reconcile: &Arc<Notify>,
    log_manager: &LogManager,
    app: &App,
    deployment: &Deployment,
) -> DeploymentResult<()> {
    let name = app_container_name(&app.id, &deployment.id);

    // container does not exist, do nothing
    if docker_client.inspect_container(&name, None).await.is_err() {
        return Ok(());
    }

    let host_port = get_container_host_port(docker_client, &name).await?;

    docker_client
        .stop_container(
            &name,
            Some(StopContainerOptionsBuilder::new().t(10).build()),
        )
        .await?;

    port_pool.release(host_port).await;

    log_manager.remove(&LogKey::Deployment {
        app_slug: app.slug.clone(),
        deployment_id: deployment.id.clone(),
    });

    DeploymentRepo::update_status(db_pool, &deployment.id, DeploymentStatus::Stopped).await?;

    proxy_reconcile.notify_one();

    Ok(())
}

pub async fn delete_deployment_container(
    docker_client: &Docker,
    port_pool: &Arc<PortPool>,
    proxy_reconcile: &Arc<Notify>,
    log_manager: &LogManager,
    app: &App,
    deployment: &Deployment,
) -> DeploymentResult<()> {
    let name = app_container_name(&app.id, &deployment.id);

    // container does not exist, do nothing
    if docker_client.inspect_container(&name, None).await.is_err() {
        return Ok(());
    }

    let host_port = if deployment.status != DeploymentStatus::Stopped {
        Some(get_container_host_port(docker_client, &name).await?)
    } else {
        None
    };

    docker_client
        .remove_container(
            &name,
            Some(RemoveContainerOptionsBuilder::new().force(true).build()),
        )
        .await?;

    if let Some(port) = host_port {
        port_pool.release(port).await;
    }

    log_manager.remove(&LogKey::Deployment {
        app_slug: app.slug.clone(),
        deployment_id: deployment.id.clone(),
    });

    proxy_reconcile.notify_one();

    Ok(())
}

pub async fn delete_app_volumes(docker_client: &Docker, app_id: &str) -> DeploymentResult<()> {
    let prefix = app_volume_prefix(app_id);

    let mut filters: HashMap<String, Vec<String>> = HashMap::new();
    filters.insert("name".to_string(), vec![prefix.clone()]);

    let opts = ListVolumesOptions {
        filters: Some(filters),
    };

    let response = docker_client.list_volumes(Some(opts)).await?;

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
            tracing::warn!("Failed to remove app volume {}: {:?}", name, e);
        }
    }

    Ok(())
}
