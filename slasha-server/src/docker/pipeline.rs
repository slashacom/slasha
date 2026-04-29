use std::{collections::HashMap, path::Path, sync::Arc};

use bollard::Docker;
use slasha_db::{
    DbPool,
    app::{App, AppEnvVar},
    deployment::{Deployment, DeploymentStatus},
    repos::{app::AppRepo, deployment::DeploymentRepo, service::ServiceRepo},
    service::{Service, ServiceStatus},
};
use tokio::sync::Notify;

use super::{
    DeploymentError, DeploymentResult,
    build::{BuildStrategy, detect_build_strategy, phase_build_docker, phase_build_railpack},
    network::app_network_name,
    run::{app_container_name, phase_run},
};
use crate::{
    docker::{
        env::{RefSource, resolve_env_value, topo_sort_vars},
        logs::{Log, LogKey},
        port_pool::PortPool,
        services::service_container_name,
    },
    state::{Runtime, Storage},
};

const DEFAULT_RAILPACK_CONTAINER_PORT: u16 = 8080;

pub async fn resolve_app_env(
    db_pool: &DbPool,
    app: &App,
    deployment: &Deployment,
    app_vars: Vec<AppEnvVar>,
    app_services: Vec<Service>,
) -> DeploymentResult<HashMap<String, String>> {
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
                    _ => {
                        let val = tokio::task::block_in_place(|| {
                            tokio::runtime::Handle::current().block_on(async {
                                ServiceRepo::get_env_var_value(db_pool, &svc.id, key).await
                            })
                        })?;
                        val.ok_or_else(|| {
                            DeploymentError::KeyNotExported(svc_name.clone(), key.to_string())
                        })
                    }
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

/// Parse `VOLUME` directives from the **final stage** of a Dockerfile.
/// Returns each declared mount path as a normalized absolute path.
///
/// Handles both syntaxes:
///   shell form  →  `VOLUME /a /b`
///   exec form   →  `VOLUME ["/a", "/b"]`
///
/// Multi-stage builds: only directives appearing after the last `FROM` are
/// returned — anything declared in a builder stage doesn't affect the
/// runtime image and shouldn't be treated as a runtime disk.
pub fn parse_volumes(dockerfile_content: &str) -> Vec<String> {
    // Find all VOLUME entries grouped by stage; keep only the final stage.
    let mut current_stage: Vec<String> = Vec::new();

    for raw in dockerfile_content.lines() {
        let line = raw.trim();
        if line.is_empty() || line.starts_with('#') {
            continue;
        }

        let upper = line.to_uppercase();

        if upper.starts_with("FROM ") {
            current_stage.clear();
            continue;
        }

        if !upper.starts_with("VOLUME") {
            continue;
        }

        // Strip the "VOLUME" keyword (handle "VOLUME ", "VOLUME[", "VOLUME\t...")
        let rest = line["VOLUME".len()..].trim_start();

        let paths = if rest.starts_with('[') {
            parse_volume_exec_form(rest)
        } else {
            parse_volume_shell_form(rest)
        };

        for p in paths {
            let p = p.trim().to_string();
            if !p.is_empty() && !current_stage.contains(&p) {
                current_stage.push(p);
            }
        }
    }

    current_stage
}

fn parse_volume_exec_form(s: &str) -> Vec<String> {
    // VOLUME ["/a", "/b"]  → strip [], split on commas, strip quotes/spaces.
    let inner = s.trim_start_matches('[').trim_end_matches(']');
    inner
        .split(',')
        .map(|part| part.trim().trim_matches(|c| c == '"' || c == '\'').to_string())
        .filter(|p| !p.is_empty())
        .collect()
}

fn parse_volume_shell_form(s: &str) -> Vec<String> {
    // VOLUME /a /b  → split on whitespace.
    s.split_whitespace().map(str::to_string).collect()
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

        DeploymentRepo::update_status(
            &storage.db_pool,
            &deployment.id,
            DeploymentStatus::Failed,
        )
        .await?;
    }

    Ok(())
}

async fn run_deployment_inner(
    docker_client: &Docker,
    db_pool: &DbPool,
    port_pool: &Arc<PortPool>,
    proxy_reconcile: &Arc<Notify>,
    app: &App,
    deployment: &Deployment,
    log: &Log,
) -> DeploymentResult<()> {
    let repo_path = Path::new(&app.repo_path);

    let strategy = detect_build_strategy(repo_path, &deployment.commit_sha).await?;

    DeploymentRepo::update_status(db_pool, &deployment.id, DeploymentStatus::Building).await?;

    let app_vars = AppRepo::get_env_vars(db_pool, &app.id).await?;
    let app_services = ServiceRepo::list_for_app(db_pool, &app.id).await?;

    let mut env_map = resolve_app_env(db_pool, app, deployment, app_vars, app_services).await?;

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

    let volume_paths = match &strategy {
        BuildStrategy::Dockerfile { content } => parse_volumes(content),
        BuildStrategy::Railpack => Vec::new(),
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
                volume_paths,
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
                volume_paths,
            )
            .await?;
        }
    }

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parse_volumes_shell_form() {
        let dockerfile = "FROM alpine\nVOLUME /data /cache\n";
        assert_eq!(parse_volumes(dockerfile), vec!["/data", "/cache"]);
    }

    #[test]
    fn parse_volumes_exec_form() {
        let dockerfile = "FROM alpine\nVOLUME [\"/data\", \"/cache\"]\n";
        assert_eq!(parse_volumes(dockerfile), vec!["/data", "/cache"]);
    }

    #[test]
    fn parse_volumes_multiple_directives() {
        let dockerfile = "FROM alpine\nVOLUME /data\nVOLUME /cache\n";
        assert_eq!(parse_volumes(dockerfile), vec!["/data", "/cache"]);
    }

    #[test]
    fn parse_volumes_only_final_stage() {
        // Volumes from the builder stage shouldn't leak into the runtime image.
        let dockerfile = "\
FROM alpine AS builder
VOLUME /build-cache
FROM alpine AS runtime
VOLUME /data
";
        assert_eq!(parse_volumes(dockerfile), vec!["/data"]);
    }

    #[test]
    fn parse_volumes_dedupes_within_stage() {
        let dockerfile = "FROM alpine\nVOLUME /data\nVOLUME /data\n";
        assert_eq!(parse_volumes(dockerfile), vec!["/data"]);
    }

    #[test]
    fn parse_volumes_no_volumes() {
        let dockerfile = "FROM alpine\nEXPOSE 8080\n";
        assert!(parse_volumes(dockerfile).is_empty());
    }

    #[test]
    fn parse_volumes_skips_comments_and_blanks() {
        let dockerfile = "\
FROM alpine
# VOLUME /should-not-be-parsed
VOLUME /data
";
        assert_eq!(parse_volumes(dockerfile), vec!["/data"]);
    }

    #[test]
    fn parse_volumes_handles_road_to_style() {
        // Mirrors ~/Vibecode/road-to/Dockerfile.
        let dockerfile = "\
FROM golang:1.25-alpine AS server-build
FROM node:20-alpine AS web-build
FROM golang:1.25-alpine AS runtime
EXPOSE 8080 3000
VOLUME [\"/app/data\"]
";
        assert_eq!(parse_volumes(dockerfile), vec!["/app/data"]);
    }
}
