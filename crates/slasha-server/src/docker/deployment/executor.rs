use std::{collections::HashMap, path::Path, sync::Arc};

use bollard::{
    Docker,
    query_parameters::{RemoveContainerOptionsBuilder, RemoveImageOptions},
};
use slasha_db::{
    DbPool,
    app::{App, AppEnvVar},
    deployment::{Deployment, DeploymentStatus, NewDeployment},
    models::app_scale::{AppScale, ProcessType},
    repos::{
        app::AppRepo, app_backup::AppBackupRepo, app_scale::AppScaleRepo,
        deployment::DeploymentRepo, service::ServiceRepo,
    },
    service::{Service, ServiceStatus},
};
use tokio::sync::Notify;
use uuid::Uuid;

use super::{
    build::{build_docker, build_railpack},
    container::{
        MANAGED_DATA_PATH, create_process_container, run_release_container, start_process_container,
    },
    dockerfile_parser::{BuildStrategy, detect_build_strategy, parse_expose, parse_volumes},
    image::{find_deployment_image, prune_app_images, tag_deployment_image},
    litestream,
    procfile_parser::{Procfile, load_procfile},
    readiness::{ReadinessConfig, ReadinessOutcome, wait_for_web_ready},
};
use crate::{
    docker::{
        DeploymentError, DeploymentResult,
        deployment::{
            container::CreateContainerContext, remove_deployment_processes,
            stop_deployment_processes,
        },
        env::{RefSource, resolve_env_value, topo_sort_vars},
        naming::{app_network_name, image_tag, process_container_name, service_container_name},
        rollback::Rollback,
    },
    logs::{LogHandle, LogKey, LogManager},
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
    let strategy = detect_build_strategy(
        Path::new(&app.repo_path),
        &app.root_dir,
        &deployment.commit_sha,
    )
    .await?;
    let app_vars = AppRepo::get_env_vars(db_pool, &app.id).await?;
    let app_services = ServiceRepo::list_for_app(db_pool, &app.id).await?;
    let mut env_map = resolve_app_env(db_pool, app, app_vars, app_services).await?;

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

    Ok(resolved)
}

struct ProcessTarget {
    process_type: ProcessType,
    command: Option<String>,
    count: u32,
}

fn resolve_process_targets(
    procfile: &Option<Procfile>,
    scale_configs: &[AppScale],
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

// executed in a background tokio task
pub async fn run_deployment(
    docker_client: Docker,
    db_pool: DbPool,
    log_manager: Arc<LogManager>,
    proxy_sync_trigger: Arc<Notify>,
    deployment_tasks: Arc<dashmap::DashMap<String, tokio_util::sync::CancellationToken>>,
    app: App,
    deployment: Deployment,
    source_image: Option<String>,
    cancel_token: tokio_util::sync::CancellationToken,
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

    let result = tokio::select! {
        res = run_deployment_inner(
            &docker_client,
            &db_pool,
            &proxy_sync_trigger,
            &log_manager,
            &deployment_tasks,
            &app,
            &deployment,
            source_image.as_deref(),
            &mut rollback,
        ) => res,
        _ = cancel_token.cancelled() => {
            Err(DeploymentError::BuildFailed("Deployment was cancelled by user".to_string()))
        }
    };

    if let Err(e) = result {
        tracing::info!(
            app_slug = %app.slug,
            deployment_id = %deployment.id,
            status = "failed",
            error = ?e,
            "deployment finish"
        );
        log.send(format!("Deployment failed: {}", e)).await?;
        log.send(
            "Rolling back this release; the previous deployment (if any) stays active".to_string(),
        )
        .await?;

        rollback.execute().await;
        log_manager.remove(&log_key);

        let _ =
            DeploymentRepo::update_status(&db_pool, &deployment.id, DeploymentStatus::Failed).await;

        deployment_tasks.remove(&deployment.id);

        return Ok(());
    }

    tracing::info!(
        app_slug = %app.slug,
        deployment_id = %deployment.id,
        status = "success",
        "deployment finish"
    );

    rollback.disarm();
    deployment_tasks.remove(&deployment.id);

    Ok(())
}

async fn run_deployment_inner(
    docker_client: &Docker,
    db_pool: &DbPool,
    proxy_sync_trigger: &Arc<Notify>,
    log_manager: &Arc<LogManager>,
    deployment_tasks: &dashmap::DashMap<String, tokio_util::sync::CancellationToken>,
    app: &App,
    deployment: &Deployment,
    source_image: Option<&str>,
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

    if let Some(source_image) = source_image {
        log.send(format!("Reusing retained image {}", source_image))
            .await?;
        tag_deployment_image(docker_client, source_image, &app.slug, &deployment.id).await?;
    } else {
        log.send(format!(
            "Building image slasha/{}:{} ({})",
            app.slug, deployment.id, build_label
        ))
        .await?;

        match &deployment_context.strategy {
            BuildStrategy::Dockerfile { .. } => {
                build_docker(docker_client, &log, app, deployment).await?
            }
            BuildStrategy::Railpack => build_railpack(docker_client, &log, app, deployment).await?,
        }
    }

    rollback.register({
        let docker_client = docker_client.clone();
        let tag = image_tag(&app.slug, &deployment.id);

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
                                "Failed to remove process container"
                            );
                        }
                    })
                }
            });

            created_containers.push((target.process_type, i));
        }
    }

    for (pt, i) in &created_containers {
        start_process_container(docker_client, &log, app, deployment, *pt, *i).await?;
    }

    enforce_web_readiness(
        docker_client,
        &log,
        app,
        deployment,
        &deployment_context,
        &created_containers,
    )
    .await?;

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

    let previous_deployments: Vec<Deployment> =
        DeploymentRepo::list_active_for_app(db_pool, &app.id)
            .await?
            .into_iter()
            .filter(|active_dep| active_dep.id != deployment.id)
            .collect();

    for previous in &previous_deployments {
        DeploymentRepo::update_status(db_pool, &previous.id, DeploymentStatus::Stopped).await?;
    }

    DeploymentRepo::update_status(db_pool, &deployment.id, DeploymentStatus::Running).await?;
    proxy_sync_trigger.notify_one();

    for previous in &previous_deployments {
        if let Err(e) = stop_deployment_processes(
            docker_client,
            db_pool,
            proxy_sync_trigger,
            log_manager,
            deployment_tasks,
            app,
            previous,
        )
        .await
        {
            tracing::warn!(
                app_slug = %app.slug,
                deployment_id = %previous.id,
                error = ?e,
                "Failed to stop previous deployment"
            );
            continue;
        }

        if let Err(e) = remove_deployment_processes(
            docker_client,
            proxy_sync_trigger,
            log_manager,
            app,
            previous,
        )
        .await
        {
            tracing::warn!(
                app_slug = %app.slug,
                deployment_id = %previous.id,
                error = ?e,
                "Failed to remove previous deployment containers"
            );
        }
    }

    if let Err(e) = prune_app_images(docker_client, db_pool, app).await {
        tracing::warn!(
            app_slug = %app.slug,
            error = ?e,
            "Failed to prune old deployment images"
        );
    }

    Ok(())
}

// Gate the release on the web process actually answering HTTP. Runs after the
// new containers start but before the previous deployment is stopped, so a
// booting-but-broken release fails (and rolls back) while the old release
// keeps serving traffic.
async fn enforce_web_readiness(
    docker_client: &Docker,
    log: &LogHandle,
    app: &App,
    deployment: &Deployment,
    deployment_context: &DeploymentContext,
    created_containers: &[(ProcessType, u32)],
) -> DeploymentResult<()> {
    let web_containers: Vec<String> = created_containers
        .iter()
        .filter(|(pt, _)| *pt == ProcessType::Web)
        .map(|(pt, i)| {
            process_container_name(&app.id, &deployment.id, &pt.to_string().to_lowercase(), *i)
        })
        .collect();

    if web_containers.is_empty() {
        return Ok(());
    }

    let is_local = app.node_id == slasha_db::models::node::LOCAL_NODE_ID;

    let config = ReadinessConfig::from_env_map(&deployment_context.env_map);

    log.send(format!(
        "Waiting for web process to respond on GET {} (timeout: {}s)",
        config.path,
        config.timeout.as_secs()
    ))
    .await?;

    for container_name in &web_containers {
        match wait_for_web_ready(
            docker_client,
            container_name,
            deployment_context.container_port,
            &config,
            is_local,
        )
        .await
        {
            ReadinessOutcome::Ready { elapsed } => {
                log.send(format!(
                    "{} became ready in {:.1}s",
                    container_name,
                    elapsed.as_secs_f64()
                ))
                .await?;
            }
            ReadinessOutcome::Unreachable => {
                tracing::warn!(
                    app_slug = %app.slug,
                    container = %container_name,
                    "Container network is unreachable from the host; skipping readiness check"
                );
                log.send(
                    "Warning: container network is unreachable from the host; skipping readiness check"
                        .to_string(),
                )
                .await?;
                break;
            }
            ReadinessOutcome::NotReady { reason } => {
                return Err(DeploymentError::AppNotReady(reason));
            }
        }
    }

    Ok(())
}

fn resolve_commit_message(repo_path: &str, sha: &str) -> DeploymentResult<String> {
    let repo = git2::Repository::open(repo_path)?;
    let commit = repo.find_commit(git2::Oid::from_str(sha)?)?;
    Ok(commit.summary().unwrap_or("").to_string())
}

pub fn resolve_head_commit(repo_path: &str, branch: &str) -> DeploymentResult<(String, String)> {
    let repo = git2::Repository::open(repo_path)?;
    let branch = repo.find_branch(branch, git2::BranchType::Local)?;
    let commit = branch.get().peel_to_commit()?;

    Ok((
        commit.id().to_string(),
        commit.summary().unwrap_or("").to_string(),
    ))
}

// starts a deployment and spawns a build
// returns none if another build is active
pub async fn trigger_deployment(
    docker_client: Docker,
    db_pool: DbPool,
    log_manager: Arc<LogManager>,
    proxy_sync_trigger: Arc<Notify>,
    deployment_tasks: Arc<dashmap::DashMap<String, tokio_util::sync::CancellationToken>>,
    app: App,
    commit_sha: Option<String>,
) -> DeploymentResult<Option<Deployment>> {
    let active_deployments = DeploymentRepo::list_active_for_app(&db_pool, &app.id).await?;
    let is_building = active_deployments
        .iter()
        .any(|d| d.status == DeploymentStatus::Building);
    if is_building {
        return Ok(None);
    }

    let (commit_sha, commit_message) = match commit_sha {
        Some(sha) => {
            let msg = resolve_commit_message(&app.repo_path, &sha)?;
            (sha, msg)
        }
        None => resolve_head_commit(&app.repo_path, &app.default_branch)?,
    };

    let deployment = NewDeployment {
        id: Uuid::new_v4().to_string(),
        app_id: app.id.clone(),
        commit_sha,
        commit_message,
        status: DeploymentStatus::Pending,
        node_id: app.node_id.clone(),
    };

    let deployment = DeploymentRepo::create(&db_pool, deployment).await?;

    let cancel_token = tokio_util::sync::CancellationToken::new();

    let _handle = tokio::spawn(run_deployment(
        docker_client,
        db_pool,
        log_manager,
        proxy_sync_trigger,
        deployment_tasks.clone(),
        app,
        deployment.clone(),
        None,
        cancel_token.clone(),
    ));

    deployment_tasks.insert(deployment.id.clone(), cancel_token);

    Ok(Some(deployment))
}

// starts a rollback from a retained image
// returns none if another build is active
pub async fn trigger_rollback(
    docker_client: Docker,
    db_pool: DbPool,
    log_manager: Arc<LogManager>,
    proxy_sync_trigger: Arc<Notify>,
    deployment_tasks: Arc<dashmap::DashMap<String, tokio_util::sync::CancellationToken>>,
    app: App,
    source_deployment: Deployment,
) -> DeploymentResult<Option<Deployment>> {
    let active_deployments = DeploymentRepo::list_active_for_app(&db_pool, &app.id).await?;
    if active_deployments
        .iter()
        .any(|deployment| deployment.status == DeploymentStatus::Building)
    {
        return Ok(None);
    }

    let source_image = find_deployment_image(&docker_client, &app, &source_deployment).await?;

    let deployment = NewDeployment {
        id: Uuid::new_v4().to_string(),
        app_id: app.id.clone(),
        commit_sha: source_deployment.commit_sha,
        commit_message: source_deployment.commit_message,
        status: DeploymentStatus::Pending,
        node_id: source_deployment.node_id,
    };

    let deployment = DeploymentRepo::create(&db_pool, deployment).await?;

    let cancel_token = tokio_util::sync::CancellationToken::new();

    let _handle = tokio::spawn(run_deployment(
        docker_client,
        db_pool,
        log_manager,
        proxy_sync_trigger,
        deployment_tasks.clone(),
        app,
        deployment.clone(),
        Some(source_image),
        cancel_token.clone(),
    ));

    deployment_tasks.insert(deployment.id.clone(), cancel_token);

    Ok(Some(deployment))
}
