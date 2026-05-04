use std::{collections::HashMap, path::Path, sync::Arc};

use bollard::{
    Docker,
    query_parameters::{RemoveContainerOptionsBuilder, RemoveImageOptions},
};
use slasha_db::{
    DbPool,
    app::{App, AppEnvVar},
    deployment::{Deployment, DeploymentStatus},
    repos::{app::AppRepo, deployment::DeploymentRepo, service::ServiceRepo},
    service::{Service, ServiceStatus},
};
use tokio::sync::Notify;

use super::{
    build::{build_docker, build_railpack},
    container::{create_deployment_container, start_deployment_container},
    dockerfile_parser::{BuildStrategy, detect_build_strategy, parse_expose, parse_volumes},
};
use crate::docker::{
    DeploymentError, DeploymentResult,
    env::{RefSource, resolve_env_value, topo_sort_vars},
    logs::{Log, LogKey, LogManager},
    naming::{app_container_name, app_network_name, image_tag, service_container_name},
    rollback::Rollback,
};

const DEFAULT_RAILPACK_CONTAINER_PORT: u16 = 8080;

pub async fn resolve_app_env(
    db_pool: &DbPool,
    app: &App,
    deployment: &Deployment,
    app_vars: Vec<AppEnvVar>,
    app_services: Vec<Service>,
) -> DeploymentResult<HashMap<String, String>> {
    let mut service_env_map: HashMap<String, HashMap<String, String>> = HashMap::new();
    for svc in &app_services {
        let vars = ServiceRepo::get_env_vars(db_pool, &svc.id).await?;
        service_env_map.insert(
            svc.id.clone(),
            vars.into_iter().map(|v| (v.key, v.value)).collect(),
        );
    }

    let sorted_vars = topo_sort_vars(app_vars, |v| &v.key, |v| &v.value)?;
    let mut resolved: HashMap<String, String> = HashMap::with_capacity(sorted_vars.len());

    for var in sorted_vars {
        let value = resolve_env_value(&var.value, |source, key| match source {
            RefSource::Own => Ok(resolved.get(key).unwrap().clone()),

            RefSource::System => match key {
                "app_container_name" => Ok(app_container_name(&app.id, &deployment.id)),
                "app_id" => Ok(app.id.clone()),
                "app_name" => Ok(app.name.clone()),
                "app_slug" => Ok(app.slug.clone()),
                "network_name" => Ok(app_network_name(&app.id)),
                _ => Err(DeploymentError::EnvResolveFailed(format!(
                    "Unknown system key: {}",
                    key
                ))),
            },

            RefSource::Service(svc_name) => {
                let svc = app_services
                    .iter()
                    .find(|s| &s.name == svc_name)
                    .ok_or_else(|| DeploymentError::ServiceNotFound(svc_name.clone()))?;

                if svc.status != ServiceStatus::Running {
                    return Err(DeploymentError::ServiceNotRunning(svc_name.clone()));
                }

                match key {
                    "service_container_name" => Ok(service_container_name(&svc.id)),
                    _ => service_env_map
                        .get(&svc.id)
                        .and_then(|m| m.get(key))
                        .cloned()
                        .ok_or_else(|| {
                            DeploymentError::KeyNotExported(svc_name.clone(), key.to_string())
                        }),
                }
            }
        })?;

        resolved.insert(var.key.clone(), value);
    }

    Ok(resolved)
}

pub async fn run_deployment(
    docker_client: Docker,
    db_pool: DbPool,
    log_manager: Arc<LogManager>,
    proxy_sync_trigger: Arc<Notify>,
    app: App,
    deployment: Deployment,
) -> DeploymentResult<()> {
    let log_key = LogKey::Deployment {
        app_slug: app.slug.clone(),
        deployment_id: deployment.id.clone(),
    };

    let log = log_manager.get_logger(&log_key).await?;
    let mut rollback = Rollback::new();

    if let Err(e) = run_deployment_inner(
        &docker_client,
        &db_pool,
        &proxy_sync_trigger,
        &app,
        &deployment,
        &log,
        &mut rollback,
    )
    .await
    {
        tracing::error!("Deployment {} failed: {:?}", deployment.id, e);
        log.send(format!("Deployment failed: {}", e)).await?;

        rollback.execute().await;
        log_manager.remove(&log_key);

        DeploymentRepo::update_status(&db_pool, &deployment.id, DeploymentStatus::Failed).await?;

        return Err(e);
    }

    rollback.disarm();
    Ok(())
}

async fn run_deployment_inner(
    docker_client: &Docker,
    db_pool: &DbPool,
    proxy_sync_trigger: &Arc<Notify>,
    app: &App,
    deployment: &Deployment,
    log: &Log,
    rollback: &mut Rollback,
) -> DeploymentResult<()> {
    let strategy = detect_build_strategy(Path::new(&app.repo_path), &deployment.commit_sha).await?;

    DeploymentRepo::update_status(db_pool, &deployment.id, DeploymentStatus::Building).await?;

    let app_vars = AppRepo::get_env_vars(db_pool, &app.id).await?;
    let app_services = ServiceRepo::list_for_app(db_pool, &app.id).await?;
    let mut env_map = resolve_app_env(db_pool, app, deployment, app_vars, app_services).await?;

    let container_port = resolve_container_port(&strategy, &mut env_map)?;
    let volume_paths = resolve_volume_paths(&strategy);

    let build_label = match strategy {
        BuildStrategy::Dockerfile { .. } => "Dockerfile",
        BuildStrategy::Railpack => "Railpack",
    };

    log.send(format!(
        "Building image slasha/{}:{} ({})",
        app.slug, deployment.commit_sha, build_label
    ))
    .await?;

    match &strategy {
        BuildStrategy::Dockerfile { .. } => {
            build_docker(docker_client, log, app, deployment).await?
        }
        BuildStrategy::Railpack => build_railpack(docker_client, log, app, deployment).await?,
    };

    rollback.register({
        let docker_client = docker_client.clone();
        let tag = image_tag(&app.slug, &deployment.commit_sha);

        move || {
            Box::pin(async move {
                let _ = docker_client
                    .remove_image(
                        &tag,
                        Some(RemoveImageOptions {
                            force: true,
                            ..Default::default()
                        }),
                        None,
                    )
                    .await;
            })
        }
    });

    let container_name = create_deployment_container(
        docker_client,
        app,
        deployment,
        container_port,
        env_map,
        volume_paths,
    )
    .await?;

    rollback.register({
        let docker_client = docker_client.clone();
        let container_name = container_name.clone();

        move || {
            Box::pin(async move {
                let _ = docker_client
                    .remove_container(
                        &container_name,
                        Some(RemoveContainerOptionsBuilder::new().force(true).build()),
                    )
                    .await;
            })
        }
    });

    start_deployment_container(
        docker_client,
        db_pool,
        proxy_sync_trigger,
        log,
        &deployment.id,
        &container_name,
    )
    .await?;

    Ok(())
}

fn resolve_container_port(
    strategy: &BuildStrategy,
    env_map: &mut HashMap<String, String>,
) -> DeploymentResult<u16> {
    if let Some(port_str) = env_map.get("PORT") {
        return port_str
            .parse::<u16>()
            .map_err(|e| DeploymentError::EnvResolveFailed(e.to_string()));
    }

    match strategy {
        BuildStrategy::Dockerfile { content } => Ok(parse_expose(content)),
        BuildStrategy::Railpack => {
            env_map.insert(
                "PORT".to_string(),
                DEFAULT_RAILPACK_CONTAINER_PORT.to_string(),
            );
            Ok(DEFAULT_RAILPACK_CONTAINER_PORT)
        }
    }
}

fn resolve_volume_paths(strategy: &BuildStrategy) -> Vec<String> {
    match strategy {
        BuildStrategy::Dockerfile { content } => parse_volumes(content),
        BuildStrategy::Railpack => Vec::new(),
    }
}
