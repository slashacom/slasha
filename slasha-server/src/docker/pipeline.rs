use std::path::Path;
use std::sync::Arc;

use bollard::Docker;
use diesel::r2d2::{ConnectionManager, Pool};
use diesel::sqlite::SqliteConnection;
use models::app::App;
use models::deployment::{Deployment, DeploymentStatus};

use super::broadcaster::DeploymentBroadcaster;
use super::build::{
    BuildStrategy, detect_build_strategy, phase_build_docker, phase_build_railpack,
};
use super::port_pool::PortPool;
use super::run::{phase_run, update_deployment_status};
use crate::Error;
use crate::error::{DeploymentError, Result};

use std::collections::HashMap;

use diesel::prelude::*;

use models::app::AppEnvVar;
use models::schema::{app_env_vars, services, service_env_vars};
use models::service::{Service, ServiceStatus};

const DEFAULT_RAILPACK_CONTAINER_PORT: u16 = 8080;

fn parse_service_ref(value: &str) -> Option<(String, String)> {
    let trimmed = value.trim();
    let inner = trimmed
        .strip_prefix("${{")
        .and_then(|s| s.strip_suffix("}}"))?
        .trim();

    let (service_name, env_key) = inner.split_once('.')?;
    Some((service_name.trim().to_string(), env_key.trim().to_string()))
}

pub fn resolve_app_env(
    db_pool: &Pool<ConnectionManager<SqliteConnection>>,
    app_id: &str,
) -> Result<HashMap<String, String>> {
    let mut conn = db_pool.get().map_err(DeploymentError::PoolError)?;

    let vars: Vec<AppEnvVar> = app_env_vars::table
        .filter(app_env_vars::app_id.eq(app_id))
        .order(app_env_vars::key.asc())
        .load(&mut conn)?;

    let app_services: Vec<Service> = services::table
        .filter(services::app_id.eq(app_id))
        .load(&mut conn)?;

    let mut resolved: HashMap<String, String> = HashMap::with_capacity(vars.len());

    for var in vars {
        let value = if let Some((svc_name, env_key)) = parse_service_ref(&var.value) {
            let svc = app_services
                .iter()
                .find(|s| s.name == svc_name)
                .ok_or_else(|| DeploymentError::ServiceNotFound(svc_name.clone()))?;

            if svc.status != ServiceStatus::Running {
                return Err(Error::Deployment(DeploymentError::ServiceNotRunning(
                    svc_name,
                )));
            }

            let svc_env: Option<String> = service_env_vars::table
                .filter(service_env_vars::service_id.eq(&svc.id))
                .filter(service_env_vars::key.eq(&env_key))
                .select(service_env_vars::value)
                .first::<String>(&mut conn)
                .optional()
                .map_err(|e| DeploymentError::DatabaseError(e))?;

            if let Some(val) = svc_env {
                val
            } else {
                return Err(Error::Deployment(DeploymentError::KeyNotExported(
                    svc_name, env_key,
                )));
            }
        } else {
            var.value.clone()
        };

        resolved.insert(var.key, value);
    }

    Ok(resolved)
}

pub fn detect_container_port(dockerfile_content: &str) -> u16 {
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
    docker: Arc<Docker>,
    pool: Arc<PortPool>,
    broadcaster: Arc<DeploymentBroadcaster>,
    db_pool: Pool<ConnectionManager<SqliteConnection>>,
    app: App,
    deployment: Deployment,
) -> Result<()> {
    if let Err(e) =
        run_deployment_inner(&docker, &pool, &broadcaster, &db_pool, &app, &deployment).await
    {
        tracing::error!("Deployment {} failed: {:?}", deployment.id, e);

        broadcaster
            .send(&deployment.id, format!("Deployment failed: {}", e))
            .await?;

        broadcaster.remove(&deployment.id);

        if let Ok(mut conn) = db_pool.get() {
            let _ = update_deployment_status(&mut conn, &deployment.id, DeploymentStatus::Failed);
        }
    }

    Ok(())
}

async fn run_deployment_inner(
    docker: &Arc<Docker>,
    pool: &Arc<PortPool>,
    broadcaster: &Arc<DeploymentBroadcaster>,
    db_pool: &Pool<ConnectionManager<SqliteConnection>>,
    app: &App,
    deployment: &Deployment,
) -> Result<()> {
    let deployment_id = &deployment.id;
    let repo_path = Path::new(&app.repo_path);

    let strategy = detect_build_strategy(repo_path, &deployment.commit_sha).await?;

    {
        let mut conn = db_pool.get().map_err(DeploymentError::PoolError)?;
        update_deployment_status(&mut conn, deployment_id, DeploymentStatus::Building)?;
    }

    let mut env_map = resolve_app_env(db_pool, &app.id)
        .map_err(|e| DeploymentError::EnvResolveFailed(e.to_string()))?;

    let container_port = match env_map.get("PORT") {
        Some(port_str) => port_str
            .parse::<u16>()
            .map_err(|e| DeploymentError::EnvResolveFailed(e.to_string()))?,
        None => match &strategy {
            BuildStrategy::Dockerfile { content } => detect_container_port(content),

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
            broadcaster
                .send(
                    deployment_id,
                    format!(
                        "Building image slasha/{}:{} (Dockerfile)",
                        app.slug, deployment.commit_sha
                    ),
                )
                .await?;

            phase_build_docker(docker, broadcaster, app, deployment).await?;

            phase_run(
                docker,
                db_pool,
                broadcaster,
                pool,
                app,
                deployment,
                container_port,
                env_map,
            )
            .await?;
        }

        BuildStrategy::Railpack => {
            broadcaster
                .send(
                    deployment_id,
                    format!(
                        "Building image slasha/{}:{} (Railpack)",
                        app.slug, deployment.commit_sha
                    ),
                )
                .await?;

            phase_build_railpack(docker, broadcaster, app, deployment).await?;

            phase_run(
                docker,
                db_pool,
                broadcaster,
                pool,
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
