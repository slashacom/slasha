use std::collections::HashMap;
use std::path::Path;
use std::sync::Arc;

use anyhow::{Context, Result};
use bollard::Docker;
use bollard::body_stream;
use bollard::models::{
    ContainerCreateBody, HostConfig, PortBinding, RestartPolicy, RestartPolicyNameEnum,
};
use bollard::query_parameters::RemoveContainerOptionsBuilder;
use bollard::query_parameters::{
    BuildImageOptionsBuilder, CreateContainerOptions, LogsOptionsBuilder,
    StartContainerOptionsBuilder, StopContainerOptionsBuilder, TagImageOptionsBuilder,
};
use bytes::Bytes;
use chrono::Utc;
use diesel::prelude::*;
use diesel::r2d2::{ConnectionManager, Pool};
use diesel::sqlite::SqliteConnection;
use futures_util::{StreamExt, stream};
use models::app::App;
use models::deployment::{Deployment, DeploymentStatus};
use models::schema::deployments;
use tokio::process::Command;

use super::broadcaster::DeploymentBroadcaster;
use super::port_pool::PortPool;
use super::utils::detect_container_port;

type DbPool = Pool<ConnectionManager<SqliteConnection>>;

fn container_name(app_id: &str, deployment_id: &str) -> String {
    format!("slasha-{}-{}", app_id, deployment_id)
}

fn image_name(app_slug: &str) -> String {
    format!("slasha/{}", app_slug)
}

fn update_deployment_status(
    conn: &mut SqliteConnection,
    deployment_id: &str,
    status: DeploymentStatus,
) -> Result<()> {
    diesel::update(deployments::table.filter(deployments::id.eq(deployment_id)))
        .set((
            deployments::status.eq(status.to_string()),
            deployments::updated_at.eq(Utc::now().naive_utc()),
        ))
        .execute(conn)
        .context("Failed to update deployment status")?;

    Ok(())
}

async fn get_container_port(docker: &Docker, name: &str) -> Result<u16> {
    let info = docker.inspect_container(name, None).await?;
    Ok(info
        .network_settings
        .and_then(|ns| ns.ports)
        .and_then(|ports| {
            ports
                .into_values()
                .flatten()
                .flatten()
                .next()
                .and_then(|pb| pb.host_port.and_then(|s| s.parse::<u16>().ok()))
        })
        .context("Failed to get container port")?)
}

async fn read_dockerfile(repo_path: &Path, commit_sha: &str) -> Result<Option<String>> {
    let out = Command::new("git")
        .arg("show")
        .arg(format!("{}:Dockerfile", commit_sha))
        .current_dir(repo_path)
        .output()
        .await
        .context("Failed to run git show for Dockerfile")?;

    if out.status.success() {
        Ok(Some(
            String::from_utf8(out.stdout).context("Dockerfile is not valid UTF-8")?,
        ))
    } else {
        Ok(None)
    }
}

async fn build_tar_context(repo_path: &Path, commit_sha: &str) -> Result<Bytes> {
    let out = Command::new("git")
        .args(["archive", "--format=tar", commit_sha])
        .current_dir(repo_path)
        .output()
        .await
        .context("Failed to run git archive")?;

    if !out.status.success() {
        let stderr = String::from_utf8_lossy(&out.stderr);
        anyhow::bail!("git archive failed: {}", stderr);
    }

    Ok(Bytes::from(out.stdout))
}

async fn phase_build(
    docker: &Docker,
    broadcaster: &DeploymentBroadcaster,
    app: &App,
    deployment: &Deployment,
) -> Result<()> {
    let repo_path = Path::new(&app.repo_path);
    let commit_sha: String = deployment.commit_sha.clone();
    let deployment_id: String = deployment.id.clone();
    let image_tag = format!("{}:{}", image_name(&app.slug), commit_sha);

    let tar_bytes = build_tar_context(repo_path, &commit_sha).await?;
    let tar_body_stream = body_stream(stream::once(async move { tar_bytes }));

    let build_opts = BuildImageOptionsBuilder::new()
        .t(image_tag.as_str())
        .rm(true)
        .forcerm(true)
        .build();

    let mut build_stream = docker.build_image(build_opts, None, Some(tar_body_stream));

    while let Some(item) = build_stream.next().await {
        match item {
            Ok(info) => {
                if let Some(line) = info.stream {
                    let line = line.trim_end_matches('\n').to_string();
                    if !line.is_empty() {
                        broadcaster.send(&deployment_id, line).await?;
                    }
                }
                if let Some(detail) = info.error_detail {
                    if let Some(msg_text) = detail.message {
                        let msg = format!("Build error: {}", msg_text.trim());
                        broadcaster.send(&deployment_id, msg.clone()).await?;
                        anyhow::bail!(msg);
                    }
                }
            }
            Err(e) => {
                let msg = format!("Docker error during build: {}", e);
                broadcaster.send(&deployment_id, msg.clone()).await?;
                anyhow::bail!(msg);
            }
        }
    }

    let latest_tag = image_name(&app.slug);
    let tag_opts = TagImageOptionsBuilder::new()
        .repo(latest_tag.as_str())
        .tag("latest")
        .build();
    docker
        .tag_image(&image_tag, Some(tag_opts))
        .await
        .context("Failed to tag image as :latest")?;

    let done_msg = format!("Image built and tagged as {}:latest", latest_tag);
    broadcaster.send(&deployment_id, done_msg).await?;

    Ok(())
}

async fn phase_run(
    docker: &Docker,
    db_pool: &DbPool,
    broadcaster: &Arc<DeploymentBroadcaster>,
    pool: &Arc<PortPool>,
    app: &App,
    deployment: &Deployment,
    container_port: u16,
) -> Result<()> {
    let deployment_id = deployment.id.clone();
    let host_port = pool.allocate().await?;
    let name = container_name(&app.id, &deployment_id);
    let image = format!("{}:{}", image_name(&app.slug), deployment.commit_sha);

    let port_key = format!("{}/tcp", container_port);
    let mut port_bindings: HashMap<String, Option<Vec<PortBinding>>> = HashMap::new();
    port_bindings.insert(
        port_key,
        Some(vec![PortBinding {
            host_ip: Some("0.0.0.0".to_string()),
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

    let container_config = ContainerCreateBody {
        image: Some(image),
        labels: Some(labels),
        host_config: Some(host_config),
        ..Default::default()
    };

    let create_opts = CreateContainerOptions {
        name: Some(name.clone()),
        ..Default::default()
    };

    docker
        .create_container(Some(create_opts), container_config)
        .await
        .context("Failed to create container")?;

    docker
        .start_container(&name, Some(StartContainerOptionsBuilder::new().build()))
        .await
        .context("Failed to start container")?;

    let mut conn = db_pool.get().context("DB pool error")?;
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
            tracing::warn!("log stream ended for: {:?}", e);
        }
    });

    Ok(())
}

async fn stream_runtime_logs(
    docker: Docker,
    broadcaster: Arc<DeploymentBroadcaster>,
    deployment_id: String,
    container: String,
) -> Result<()> {
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

pub async fn run_deployment(
    docker: Arc<Docker>,
    pool: Arc<PortPool>,
    broadcaster: Arc<DeploymentBroadcaster>,
    db_pool: DbPool,
    app: App,
    deployment: Deployment,
) -> Result<()> {
    if let Err(e) =
        run_deployment_inner(&docker, &pool, &broadcaster, &db_pool, &app, &deployment).await
    {
        tracing::error!("Deployment {} failed: {:?}", deployment.id, e);

        let msg = format!("Deployment failed: {}", e);
        broadcaster.send(&deployment.id, msg.clone()).await?;
        broadcaster.remove(&deployment.id);

        if let Ok(mut conn) = db_pool.get() {
            update_deployment_status(&mut conn, &deployment.id, DeploymentStatus::Failed)?;
        }
    }

    Ok(())
}

async fn run_deployment_inner(
    docker: &Arc<Docker>,
    pool: &Arc<PortPool>,
    broadcaster: &Arc<DeploymentBroadcaster>,
    db_pool: &DbPool,
    app: &App,
    deployment: &Deployment,
) -> Result<()> {
    let deployment_id = &deployment.id;
    let repo_path = Path::new(&app.repo_path);

    let dockerfile_content = read_dockerfile(repo_path, &deployment.commit_sha)
        .await?
        .ok_or_else(|| {
            anyhow::anyhow!(
                "No Dockerfile found at root of commit {}. Slasha requires a Dockerfile.",
                deployment.commit_sha
            )
        })?;

    let container_port = detect_container_port(&dockerfile_content);

    {
        let mut conn = db_pool.get().context("DB pool error")?;
        update_deployment_status(&mut conn, deployment_id, DeploymentStatus::Building)?;
    }

    let building_msg = format!(
        "Building image slasha/{}:{}",
        app.slug, deployment.commit_sha
    );
    broadcaster
        .send(deployment_id, building_msg.clone())
        .await?;

    phase_build(&docker, &broadcaster, &app, &deployment).await?;

    phase_run(
        &docker,
        &db_pool,
        &broadcaster,
        &pool,
        &app,
        &deployment,
        container_port,
    )
    .await?;

    Ok(())
}

pub async fn stop_deployment_container(
    docker: &Docker,
    pool: &PortPool,
    db_pool: &DbPool,
    broadcaster: &DeploymentBroadcaster,
    app: &App,
    deployment: &Deployment,
) -> Result<()> {
    let name = container_name(&app.id, &deployment.id);

    let host_port = get_container_port(&docker, &name).await?;

    docker
        .stop_container(
            &name,
            Some(StopContainerOptionsBuilder::new().t(10).build()),
        )
        .await?;

    pool.release(host_port).await;

    broadcaster.remove(&deployment.id);

    let mut conn = db_pool.get().context("DB pool error")?;
    update_deployment_status(&mut conn, &deployment.id, DeploymentStatus::Stopped)?;

    Ok(())
}

pub async fn delete_deployment_container(
    docker: &Docker,
    pool: &PortPool,
    broadcaster: &DeploymentBroadcaster,
    app: &App,
    deployment: &Deployment,
) -> Result<()> {
    let name = container_name(&app.id, &deployment.id);

    let host_port = if deployment.status != DeploymentStatus::Stopped {
        Some(get_container_port(docker, &name).await?)
    } else {
        None
    };

    docker
        .remove_container(
            &name,
            Some(RemoveContainerOptionsBuilder::new().force(true).build()),
        )
        .await?;

    if let Some(port) = host_port {
        pool.release(port).await;
    }

    broadcaster.delete_logs(&deployment.id).await?;

    Ok(())
}
