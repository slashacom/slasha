use std::collections::HashMap;
use std::sync::Arc;

use bollard::Docker;
use bollard::models::{
    ContainerCreateBody, EndpointSettings, HostConfig, NetworkingConfig, PortBinding,
    RestartPolicy, RestartPolicyNameEnum,
};
use bollard::query_parameters::{
    CreateContainerOptions, LogsOptionsBuilder, RemoveContainerOptionsBuilder,
    StartContainerOptionsBuilder, StopContainerOptionsBuilder,
};
use chrono::Utc;
use diesel::prelude::*;
use diesel::r2d2::{ConnectionManager, Pool};
use diesel::sqlite::SqliteConnection;
use futures_util::StreamExt;
use models::app::App;
use models::deployment::{Deployment, DeploymentStatus};
use models::schema::deployments;

use super::DeploymentResult;
use super::broadcaster::DeploymentBroadcaster;
use super::network::app_network_name;
use super::port_pool::PortPool;
use crate::error::DeploymentError;

pub fn app_container_name(app_id: &str, deployment_id: &str) -> String {
    format!("slasha-{}-{}", app_id, deployment_id)
}

fn image_name(app_slug: &str) -> String {
    format!("slasha/{}", app_slug)
}

pub fn update_deployment_status(
    conn: &mut SqliteConnection,
    deployment_id: &str,
    status: DeploymentStatus,
) -> DeploymentResult<()> {
    diesel::update(deployments::table.filter(deployments::id.eq(deployment_id)))
        .set((
            deployments::status.eq(status.to_string()),
            deployments::updated_at.eq(Utc::now().naive_utc()),
        ))
        .execute(conn)
        .map_err(DeploymentError::DatabaseError)?;

    Ok(())
}

async fn get_container_host_port(docker: &Docker, name: &str) -> DeploymentResult<u16> {
    let info = docker
        .inspect_container(name, None)
        .await
        .map_err(DeploymentError::DockerApi)?;

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
                .into()
        })
}

pub async fn phase_run(
    docker: &Docker,
    db_pool: &Pool<ConnectionManager<SqliteConnection>>,
    broadcaster: &Arc<DeploymentBroadcaster>,
    port_pool: &Arc<PortPool>,
    app: &App,
    deployment: &Deployment,
    container_port: u16,
    env_map: HashMap<String, String>,
) -> DeploymentResult<()> {
    let deployment_id = deployment.id.clone();
    let host_port = port_pool.allocate().await?;
    let name = app_container_name(&app.id, &deployment_id);
    let image = format!("{}:{}", image_name(&app.slug), deployment.commit_sha);

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

    let host_config = HostConfig {
        port_bindings: Some(port_bindings),
        restart_policy: Some(RestartPolicy {
            name: Some(RestartPolicyNameEnum::UNLESS_STOPPED),
            maximum_retry_count: None,
        }),
        ..Default::default()
    };

    let mut conn = db_pool.get().map_err(DeploymentError::PoolError)?;

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

    docker
        .create_container(Some(create_opts), container_config)
        .await
        .map_err(DeploymentError::DockerApi)?;

    docker
        .start_container(&name, Some(StartContainerOptionsBuilder::new().build()))
        .await
        .map_err(DeploymentError::DockerApi)?;

    update_deployment_status(&mut conn, &deployment_id, DeploymentStatus::Running)?;

    let started_msg = format!("Container {} started on host port {}", name, host_port);
    broadcaster.send(&deployment_id, started_msg).await?;

    let docker_clone = docker.clone();
    let broadcaster_clone = broadcaster.clone();
    let deployment_id_clone = deployment_id.clone();
    let name_clone = name.clone();

    tokio::spawn(async move {
        if let Err(e) = stream_runtime_logs(
            docker_clone,
            broadcaster_clone,
            deployment_id_clone,
            name_clone,
        )
        .await
        {
            tracing::warn!("log stream ended with error: {:?}", e);
        }
    });

    Ok(())
}

async fn stream_runtime_logs(
    docker: Docker,
    broadcaster: Arc<DeploymentBroadcaster>,
    deployment_id: String,
    container: String,
) -> DeploymentResult<()> {
    let opts = LogsOptionsBuilder::new()
        .follow(true)
        .stdout(true)
        .stderr(true)
        .build();

    let mut log_stream = docker.logs(&container, Some(opts));
    let mut buffer = String::new();

    while let Some(item) = log_stream.next().await {
        match item {
            Ok(output) => {
                let chunk = output.to_string();
                buffer.push_str(&chunk);

                while let Some(pos) = buffer.find('\n') {
                    let line = buffer[..pos].to_string();
                    buffer.drain(..=pos);
                    broadcaster.send(&deployment_id, line).await?;
                }
            }
            Err(e) => {
                let msg = format!(
                    "Runtime log stream error for deployment {}: {}",
                    deployment_id, e
                );
                tracing::warn!("{}", msg);
                broadcaster.send(&deployment_id, msg).await?;
                break;
            }
        }
    }

    if !buffer.is_empty() {
        broadcaster.send(&deployment_id, buffer).await?;
    }

    tracing::info!("Runtime log stream ended for deployment {}", deployment_id);
    broadcaster.remove(&deployment_id);

    Ok(())
}

pub async fn stop_deployment_container(
    docker: &Docker,
    pool: &PortPool,
    db_pool: &Pool<ConnectionManager<SqliteConnection>>,
    broadcaster: &DeploymentBroadcaster,
    app: &App,
    deployment: &Deployment,
) -> DeploymentResult<()> {
    let name = app_container_name(&app.id, &deployment.id);

    let host_port = get_container_host_port(docker, &name).await?;

    docker
        .stop_container(
            &name,
            Some(StopContainerOptionsBuilder::new().t(10).build()),
        )
        .await
        .map_err(DeploymentError::DockerApi)?;

    pool.release(host_port).await;
    broadcaster.remove(&deployment.id);

    let mut conn = db_pool.get().map_err(DeploymentError::PoolError)?;
    update_deployment_status(&mut conn, &deployment.id, DeploymentStatus::Stopped)?;

    Ok(())
}

pub async fn delete_deployment_container(
    docker: &Docker,
    pool: &PortPool,
    broadcaster: &DeploymentBroadcaster,
    app: &App,
    deployment: &Deployment,
) -> DeploymentResult<()> {
    let name = app_container_name(&app.id, &deployment.id);

    // container does not exist, do nothing
    if docker.inspect_container(&name, None).await.is_err() {
        return Ok(());
    }

    let host_port = if deployment.status != DeploymentStatus::Stopped {
        Some(get_container_host_port(docker, &name).await?)
    } else {
        None
    };

    docker
        .remove_container(
            &name,
            Some(RemoveContainerOptionsBuilder::new().force(true).build()),
        )
        .await
        .map_err(DeploymentError::DockerApi)?;

    if let Some(port) = host_port {
        pool.release(port).await;
    }

    broadcaster.delete_logs(&deployment.id).await?;

    Ok(())
}
