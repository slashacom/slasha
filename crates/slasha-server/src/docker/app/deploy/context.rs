use std::path::Path;

use slasha_db::{app::App, deployment::Deployment};

use crate::docker::{
    DockerError, DockerResult,
    app::{
        env::resolve_app_env,
        parser::{
            BuildStrategy, Procfile, detect_build_strategy, parse_expose, parse_volumes,
            read_procfile,
        },
    },
};

pub const DEFAULT_CONTAINER_PORT: u16 = 8080;
pub const MANAGED_DATA_PATH: &str = "/data";

/// Resolves environment configuration, build strategy, ports, and volumes for a deployment.
///
/// # Arguments
///
/// * `db_pool` - Database connection pool reference ([`DbPool`](slasha_db::DbPool)).
/// * `app` - Target application model ([`App`]).
/// * `deployment` - Target deployment model ([`Deployment`]).
///
/// # Returns
///
/// A [`DockerResult`] containing the populated [`DeploymentContext`].
pub async fn resolve_deployment_context(
    db_pool: &slasha_db::DbPool,
    app: &App,
    deployment: &Deployment,
) -> DockerResult<DeploymentContext> {
    let strategy = detect_build_strategy(
        Path::new(&app.repo_path),
        &deployment.commit_sha,
        &app.root_dir,
    )
    .await?;
    let mut env_map = resolve_app_env(db_pool, app).await?;

    let container_port = resolve_container_port(&strategy, &mut env_map)?;
    let volume_paths = resolve_volume_paths(&strategy);
    let procfile = read_procfile(
        Path::new(&app.repo_path),
        &deployment.commit_sha,
        &app.root_dir,
    )
    .await?;

    Ok(DeploymentContext {
        strategy,
        env_map,
        container_port,
        volume_paths,
        procfile,
    })
}

/// Resolves the Git commit summary message for a commit SHA in a local repository.
///
/// # Arguments
///
/// * `repo_path` - Path to the local Git repository directory.
/// * `sha` - Commit SHA string to inspect.
///
/// # Returns
///
/// A [`DockerResult`] containing the commit summary message string.
pub fn resolve_commit_message(repo_path: &str, sha: &str) -> DockerResult<String> {
    let repo = git2::Repository::open(repo_path)?;
    let commit = repo.find_commit(git2::Oid::from_str(sha)?)?;
    Ok(commit.summary().unwrap_or("").to_string())
}

/// Resolves the HEAD commit SHA and summary message for a Git branch.
///
/// # Arguments
///
/// * `repo_path` - Path to the local Git repository directory.
/// * `branch` - Name of the local Git branch.
///
/// # Returns
///
/// A [`DockerResult`] containing a tuple `(commit_sha, commit_message)`.
pub fn resolve_head_commit(repo_path: &str, branch: &str) -> DockerResult<(String, String)> {
    let repo = git2::Repository::open(repo_path)?;
    let branch = repo.find_branch(branch, git2::BranchType::Local)?;
    let commit = branch.get().peel_to_commit()?;

    Ok((
        commit.id().to_string(),
        commit.summary().unwrap_or("").to_string(),
    ))
}

pub struct DeploymentContext {
    pub strategy: BuildStrategy,
    pub env_map: std::collections::HashMap<String, String>,
    pub container_port: u16,
    pub volume_paths: Vec<String>,
    pub procfile: Option<Procfile>,
}

/// Determines the container port from environment variables (`PORT`) or Dockerfile `EXPOSE` directives, defaulting to [`DEFAULT_CONTAINER_PORT`].
///
/// # Arguments
///
/// * `strategy` - Detected build strategy ([`BuildStrategy`]).
/// * `env_map` - Resolved environment variable map.
///
/// # Returns
///
/// A [`DockerResult`] containing the container port number (`u16`).
fn resolve_container_port(
    strategy: &BuildStrategy,
    env_map: &mut std::collections::HashMap<String, String>,
) -> DockerResult<u16> {
    if let Some(port_str) = env_map.get("PORT") {
        let port = port_str
            .parse::<u16>()
            .map_err(|e| DockerError::EnvResolveFailed(e.to_string()))?;

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

/// Extracts persistent volume mount paths declared within a Dockerfile `VOLUME` directive.
///
/// # Arguments
///
/// * `strategy` - Detected build strategy ([`BuildStrategy`]).
///
/// # Returns
///
/// A vector of container path strings.
fn resolve_volume_paths(strategy: &BuildStrategy) -> Vec<String> {
    match strategy {
        BuildStrategy::Dockerfile { content } => parse_volumes(content),
        BuildStrategy::Railpack => Vec::new(),
    }
}
