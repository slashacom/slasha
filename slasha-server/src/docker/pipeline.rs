use bollard::Docker;
use diesel::prelude::*;
use diesel::r2d2::{ConnectionManager, Pool};
use diesel::sqlite::SqliteConnection;
use models::app::{App, AppEnvVar};
use models::deployment::{Deployment, DeploymentStatus};
use models::schema::{app_env_vars, service_env_vars, services};
use models::service::{Service, ServiceStatus};
use std::collections::HashMap;
use std::path::Path;

use super::DeploymentResult;
use super::build::{
    BuildStrategy, detect_build_strategy, phase_build_docker, phase_build_railpack,
};
use super::network::app_network_name;
use super::run::{app_container_name, phase_run, update_deployment_status};
use crate::docker::env::{RefSource, resolve_env_value, topo_sort_vars};
use crate::docker::logs::{Log, LogKey};
use crate::docker::port_pool::PortPool;
use crate::docker::services::service_container_name;
use crate::error::DeploymentError;
use crate::state::{Runtime, Storage};
use std::sync::Arc;
use tokio::sync::Notify;

const DEFAULT_RAILPACK_CONTAINER_PORT: u16 = 8080;

pub fn resolve_app_env(
    db_pool: &Pool<ConnectionManager<SqliteConnection>>,
    app: &App,
    deployment: &Deployment,
) -> DeploymentResult<HashMap<String, String>> {
    let mut conn = db_pool.get().map_err(DeploymentError::PoolError)?;

    let sorted_vars = topo_sort_vars(
        app_env_vars::table
            .filter(app_env_vars::app_id.eq(&app.id))
            .order(app_env_vars::key.asc())
            .load::<AppEnvVar>(&mut conn)?,
        |v| &v.key,
        |v| &v.value,
    )?;

    let app_services: Vec<Service> = services::table
        .filter(services::app_id.eq(&app.id))
        .load(&mut conn)?;

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
                    _ => service_env_vars::table
                        .filter(service_env_vars::service_id.eq(&svc.id))
                        .filter(service_env_vars::key.eq(key))
                        .select(service_env_vars::value)
                        .first::<String>(&mut conn)
                        .optional()
                        .map_err(DeploymentError::DatabaseError)?
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

pub fn parse_expose(dockerfile_content: &str) -> u16 {
    for line in dockerfile_content.lines() {
        let trimmed = line.trim();
        if trimmed.to_uppercase().starts_with("EXPOSE ") {
            let rest = trimmed["EXPOSE ".len()..].trim();
            let port_str = rest.split('/').next().unwrap_or("").trim();
            if let Ok(port) = port_str.parse::<u16>() {
                return port;
            }
        }
    }

    8080
}

pub async fn run_deployment(
    docker_client: Docker,
    storage: Storage,
    runtime: Runtime,
    app: App,
    deployment: Deployment,
) -> DeploymentResult<()> {
    let log_key = LogKey::Deployment {
        app_slug: app.slug.clone(),
        deployment_id: deployment.id.clone(),
    };

    let log = runtime.log_manager.get_logger(&log_key).await?;

    if let Err(e) = run_deployment_inner(
        &docker_client,
        &storage.db_pool,
        &runtime.port_pool,
        &runtime.proxy_reconcile,
        &app,
        &deployment,
        &log,
    )
    .await
    {
        tracing::error!("Deployment {} failed: {:?}", deployment.id, e);
        log.send(format!("Deployment failed: {}", e)).await?;
        runtime.log_manager.remove(&log_key);

        if let Ok(mut conn) = storage.db_pool.get() {
            let _ = update_deployment_status(&mut conn, &deployment.id, DeploymentStatus::Failed);
        }
    }

    Ok(())
}

async fn run_deployment_inner(
    docker_client: &Docker,
    db_pool: &Pool<ConnectionManager<SqliteConnection>>,
    port_pool: &Arc<PortPool>,
    proxy_reconcile: &Arc<Notify>,
    app: &App,
    deployment: &Deployment,
    log: &Log,
) -> DeploymentResult<()> {
    let repo_path = Path::new(&app.repo_path);

    let strategy = detect_build_strategy(repo_path, &deployment.commit_sha).await?;

    {
        let mut conn = db_pool.get().map_err(DeploymentError::PoolError)?;
        update_deployment_status(&mut conn, &deployment.id, DeploymentStatus::Building)?;
    }

    let mut env_map = resolve_app_env(db_pool, app, deployment)?;

    let container_port = match env_map.get("PORT") {
        Some(port_str) => port_str
            .parse::<u16>()
            .map_err(|e| DeploymentError::EnvResolveFailed(e.to_string()))?,
        None => match &strategy {
            BuildStrategy::Dockerfile { content } => parse_expose(content),

            BuildStrategy::Railpack => {
                env_map.insert(
                    "PORT".to_string(),
                    DEFAULT_RAILPACK_CONTAINER_PORT.to_string(),
                );
                DEFAULT_RAILPACK_CONTAINER_PORT
            }
        },
    };

    match strategy {
        BuildStrategy::Dockerfile { content: _ } => {
            log.send(format!(
                "Building image slasha/{}:{} (Dockerfile)",
                app.slug, deployment.commit_sha
            ))
            .await?;

            phase_build_docker(docker_client, log, app, deployment).await?;

            phase_run(
                docker_client,
                db_pool,
                port_pool,
                proxy_reconcile,
                log,
                app,
                deployment,
                container_port,
                env_map,
            )
            .await?;
        }

        BuildStrategy::Railpack => {
            log.send(format!(
                "Building image slasha/{}:{} (Railpack)",
                app.slug, deployment.commit_sha
            ))
            .await?;

            phase_build_railpack(docker_client, log, app, deployment).await?;

            phase_run(
                docker_client,
                db_pool,
                port_pool,
                proxy_reconcile,
                log,
                app,
                deployment,
                container_port,
                env_map,
            )
            .await?;
        }
    }

    Ok(())
}
