use std::{collections::HashMap, path::Path, sync::Arc};

use bollard::{
    Docker,
    query_parameters::{RemoveContainerOptionsBuilder, RemoveImageOptions},
};
use slasha_db::{
    DbPool,
    app::{App, AppEnvVar},
    deployment::{Deployment, DeploymentStatus},
    models::app_scale::ProcessType,
    repos::{
        app::AppRepo, app_backup::AppBackupRepo, app_scale::AppScaleRepo,
        deployment::DeploymentRepo, service::ServiceRepo,
    },
    service::{Service, ServiceStatus},
};
use tokio::sync::Notify;

use super::{
    build::{build_docker, build_railpack},
    container::{
        MANAGED_DATA_PATH, create_process_container, run_release_container, start_process_container,
    },
    dockerfile_parser::{BuildStrategy, detect_build_strategy, parse_expose, parse_volumes},
    litestream,
    procfile_parser::{Procfile, load_procfile},
};
use crate::docker::{
    DeploymentError, DeploymentResult,
    deployment::{container::CreateContainerContext, stop_deployment_processes},
    env::{RefSource, resolve_env_value, topo_sort_vars},
    logs::{LogKey, LogManager},
    naming::{app_network_name, image_tag, process_container_name, service_container_name},
    rollback::Rollback,
};

pub const DEFAULT_CONTAINER_PORT: u16 = 8080;

pub struct DeploymentContext {
    pub strategy: BuildStrategy,
    pub env_map: HashMap<String, String>,
    pub container_port: u16,
    pub volume_paths: Vec<String>,
    pub procfile: Option<super::procfile_parser::Procfile>,
}

pub async fn resolve_deployment_context(
    db_pool: &DbPool,
    app: &App,
    deployment: &Deployment,
) -> DeploymentResult<DeploymentContext> {
    let strategy = detect_build_strategy(Path::new(&app.repo_path), &deployment.commit_sha).await?;
    let app_vars = AppRepo::get_env_vars(db_pool, &app.id).await?;
    let app_services = ServiceRepo::list_for_app(db_pool, &app.id).await?;
    let mut env_map = resolve_app_env(db_pool, app, deployment, app_vars, app_services).await?;

    let container_port = resolve_container_port(&strategy, &mut env_map)?;
    let volume_paths = resolve_volume_paths(&strategy);
    let procfile = load_procfile(Path::new(&app.repo_path), &deployment.commit_sha).await?;

    Ok(DeploymentContext {
        strategy,
        env_map,
        container_port,
        volume_paths,
        procfile,
    })
}

fn resolve_container_port(
    strategy: &BuildStrategy,
    env_map: &mut HashMap<String, String>,
) -> DeploymentResult<u16> {
    if let Some(port_str) = env_map.get("PORT") {
        let port = port_str
            .parse::<u16>()
            .map_err(|e| DeploymentError::EnvResolveFailed(e.to_string()))?;

        return Ok(port);
    }

    let port = match strategy {
        BuildStrategy::Dockerfile { content } => parse_expose(content),
        BuildStrategy::Railpack => None,
    };

    let port = port.unwrap_or(DEFAULT_CONTAINER_PORT);

    env_map.insert("PORT".to_string(), port.to_string());

    Ok(port)
}

fn resolve_volume_paths(strategy: &BuildStrategy) -> Vec<String> {
    match strategy {
        BuildStrategy::Dockerfile { content } => parse_volumes(content),
        BuildStrategy::Railpack => Vec::new(),
    }
}

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
                "app_container_name" => {
                    Ok(process_container_name(&app.id, &deployment.id, "web", 0))
                }
                "app_id" => Ok(app.id.clone()),
                "app_name" => Ok(app.name.clone()),
                "app_slug" => Ok(app.slug.clone()),
                "data_dir" => Ok(MANAGED_DATA_PATH.to_string()),
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

    resolved.insert("SLASHA_DATA_DIR".to_string(), MANAGED_DATA_PATH.to_string());

    Ok(resolved)
}

struct ProcessTarget {
    process_type: ProcessType,
    command: Option<String>,
    count: u32,
}

fn resolve_process_targets(
    procfile: &Option<Procfile>,
    scale_configs: &[slasha_db::models::app_scale::AppScale],
) -> Vec<ProcessTarget> {
    let mut targets = Vec::new();

    if let Some(pf) = procfile {
        for (pt, cmd) in &pf.commands {
            if *pt == ProcessType::Release {
                continue;
            }

            let count = scale_configs
                .iter()
                .find(|s| s.process_type == *pt)
                .map(|s| s.desired as u32)
                .unwrap_or(1);

            targets.push(ProcessTarget {
                process_type: *pt,
                command: Some(cmd.clone()),
                count,
            });
        }
    } else {
        targets.push(ProcessTarget {
            process_type: ProcessType::Web,
            command: None,
            count: 1,
        });
    }

    targets
}

pub async fn run_deployment(
    docker_client: Docker,
    db_pool: DbPool,
    log_manager: Arc<LogManager>,
    proxy_sync_trigger: Arc<Notify>,
    app: App,
    deployment: Deployment,
) -> DeploymentResult<()> {
    tracing::info!(
        app_slug = %app.slug,
        deployment_id = %deployment.id,
        "deployment start"
    );

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
        &log_manager,
        &app,
        &deployment,
        &mut rollback,
    )
    .await
    {
        tracing::info!(
            app_slug = %app.slug,
            deployment_id = %deployment.id,
            status = "failed",
            error = ?e,
            "deployment finish"
        );
        log.send(format!("Deployment failed: {}", e)).await?;

        rollback.execute().await;
        log_manager.remove(&log_key);

        let _ =
            DeploymentRepo::update_status(&db_pool, &deployment.id, DeploymentStatus::Failed).await;

        return Ok(());
    }

    tracing::info!(
        app_slug = %app.slug,
        deployment_id = %deployment.id,
        status = "success",
        "deployment finish"
    );

    rollback.disarm();
    Ok(())
}

async fn run_deployment_inner(
    docker_client: &Docker,
    db_pool: &DbPool,
    proxy_sync_trigger: &Arc<Notify>,
    log_manager: &Arc<LogManager>,
    app: &App,
    deployment: &Deployment,
    rollback: &mut Rollback,
) -> DeploymentResult<()> {
    let deployment_context = resolve_deployment_context(db_pool, app, deployment).await?;

    let log_key = LogKey::Deployment {
        app_slug: app.slug.clone(),
        deployment_id: deployment.id.clone(),
    };
    let log = log_manager.get_logger(&log_key).await?;

    // building
    DeploymentRepo::update_status(db_pool, &deployment.id, DeploymentStatus::Building).await?;

    let build_label = match deployment_context.strategy {
        BuildStrategy::Dockerfile { .. } => "Dockerfile",
        BuildStrategy::Railpack => "Railpack",
    };

    log.send(format!(
        "Building image slasha/{}:{} ({})",
        app.slug, deployment.commit_sha, build_label
    ))
    .await?;

    match &deployment_context.strategy {
        BuildStrategy::Dockerfile { .. } => {
            build_docker(docker_client, &log, app, deployment).await?
        }
        BuildStrategy::Railpack => build_railpack(docker_client, &log, app, deployment).await?,
    };

    rollback.register({
        let docker_client = docker_client.clone();
        let tag = image_tag(&app.slug, &deployment.commit_sha);

        move || {
            Box::pin(async move {
                if let Err(e) = docker_client
                    .remove_image(
                        &tag,
                        Some(RemoveImageOptions {
                            force: true,
                            ..Default::default()
                        }),
                        None,
                    )
                    .await
                {
                    tracing::warn!(image_tag = %tag, error = ?e, "Failed to remove image");
                }
            })
        }
    });

    // release
    if let Some(pf) = &deployment_context.procfile
        && let Some(cmd) = pf.get(&ProcessType::Release)
    {
        run_release_container(
            docker_client,
            &log,
            app,
            deployment,
            cmd.to_string(),
            deployment_context.env_map.clone(),
        )
        .await?;
    }

    // running processes
    let scale_configs = AppScaleRepo::list_for_app(db_pool, &app.id).await?;
    let targets = resolve_process_targets(&deployment_context.procfile, &scale_configs);

    let backup = AppBackupRepo::get(db_pool, &app.id).await.ok().flatten();

    let litestream_volume = if backup.as_ref().is_some_and(|b| b.enabled) {
        match litestream::ensure_litestream_volume(docker_client).await {
            Ok(volume) => Some(volume),
            Err(e) => {
                let _ = log
                    .send(format!(
                        "Warning: could not prepare litestream binary, skipping backups: {e}"
                    ))
                    .await;
                None
            }
        }
    } else {
        None
    };

    let mut created_containers = Vec::new();

    for target in targets {
        for i in 0..target.count {
            create_process_container(
                docker_client,
                app,
                deployment,
                CreateContainerContext {
                    process_type: target.process_type,
                    instance_index: i,
                    container_port: Some(deployment_context.container_port),
                    cmd: target.command.clone(),
                    env_map: deployment_context.env_map.clone(),
                    volume_paths: deployment_context.volume_paths.clone(),
                    backup: backup.clone(),
                    litestream_volume: litestream_volume.clone(),
                },
            )
            .await?;

            rollback.register({
                let docker_client = docker_client.clone();
                let container_name = process_container_name(
                    &app.id,
                    &deployment.id,
                    &target.process_type.to_string().to_lowercase(),
                    i,
                );

                move || {
                    Box::pin(async move {
                        if let Err(e) = docker_client
                            .remove_container(
                                &container_name,
                                Some(RemoveContainerOptionsBuilder::new().force(true).build()),
                            )
                            .await
                        {
                            tracing::warn!(
                                container = %container_name,
                                error = ?e,
                                "Failed to remove service container"
                            );
                        }
                    })
                }
            });

            created_containers.push((target.process_type, i));
        }
    }

    for (pt, i) in created_containers {
        start_process_container(docker_client, &log, app, deployment, pt, i).await?;
    }

    // A pending restore is consumed by the new web container's boot; clear the
    // flag so subsequent deploys don't keep discarding the live database.
    if backup
        .as_ref()
        .is_some_and(|b| b.enabled && b.restore_pending)
    {
        let _ = log
            .send("Restored SQLite database from backup replica".to_string())
            .await;
        if let Err(e) = AppBackupRepo::set_restore_pending(db_pool, &app.id, false).await {
            tracing::warn!(app_id = %app.id, error = ?e, "Failed to clear restore_pending");
        }
    }

    let active_deployments = DeploymentRepo::list_active_for_app(db_pool, &app.id).await?;
    for active_dep in active_deployments {
        if active_dep.id == deployment.id {
            continue;
        }

        if let Err(e) = stop_deployment_processes(
            docker_client,
            db_pool,
            proxy_sync_trigger,
            log_manager,
            app,
            &active_dep,
        )
        .await
        {
            tracing::warn!(
                app_slug = %app.slug,
                deployment_id = %active_dep.id,
                error = ?e,
                "Failed to stop previous deployment"
            );
        }
    }

    DeploymentRepo::update_status(db_pool, &deployment.id, DeploymentStatus::Running).await?;
    proxy_sync_trigger.notify_one();

    Ok(())
}
