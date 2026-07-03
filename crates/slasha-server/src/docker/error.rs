use thiserror::Error;

use crate::error::HttpError;

#[derive(Debug, Error)]
pub enum DeploymentError {
    #[error("Database error: {0}")]
    Db(#[from] slasha_db::DbError),

    #[error("git archive failed: {0}")]
    GitArchiveFailed(String),

    #[error("Git error: {0}")]
    GitError(#[from] git2::Error),

    #[error("Dockerfile is not valid UTF-8")]
    DockerfileEncoding,

    #[error("Build failed: {0}")]
    BuildFailed(String),

    #[error("railpack prepare failed with exit status {0}")]
    RailpackPrepareFailed(std::process::ExitStatus),

    #[error("docker buildx build failed with exit status {0}")]
    BuildKitFailed(std::process::ExitStatus),

    #[error("{phase} failed with exit status {status}")]
    PhaseFailed {
        phase: String,
        status: std::process::ExitStatus,
    },

    #[error("Docker API error: {0}")]
    DockerApi(#[from] bollard::errors::Error),

    #[error("Service \"{0}\" not found")]
    ServiceNotFound(String),

    #[error("Service \"{0}\" is not running")]
    ServiceNotRunning(String),

    #[error("Service \"{0}\" does not export env key \"{1}\"")]
    KeyNotExported(String, String),

    #[error("Env resolve failed: {0}")]
    EnvResolveFailed(String),

    #[error("Port allocation failed: {0}")]
    PortAllocationFailed(String),

    #[error("I/O error: {0}")]
    Io(#[from] std::io::Error),

    #[error("spawn_blocking panicked")]
    SpawnBlockingPanicked,

    #[error("Path is not valid UTF-8")]
    PathNotUtf8,

    #[error("Proxy error: {0}")]
    Proxy(#[from] crate::proxy::error::ProxyError),

    #[error("Scale error: {0}")]
    ScaleError(String),

    #[error("Release command failed with exit code {0}")]
    ReleaseFailed(i64),

    #[error("Service \"{0}\" did not become healthy within {1}s")]
    HealthcheckTimeout(String, u64),

    #[error("Service \"{0}\" reported unhealthy")]
    HealthcheckFailed(String),

    #[error("App failed readiness check: {0}")]
    AppNotReady(String),
}

pub type DeploymentResult<T> = std::result::Result<T, DeploymentError>;

impl From<DeploymentError> for HttpError {
    fn from(e: DeploymentError) -> Self {
        match e {
            DeploymentError::ServiceNotFound(msg) => HttpError::not_found(msg),
            DeploymentError::ServiceNotRunning(msg) => {
                HttpError::bad_request(format!("Service {} is not running", msg))
            }
            DeploymentError::KeyNotExported(svc, key) => {
                HttpError::bad_request(format!("Service {} does not export key {}", svc, key))
            }
            DeploymentError::ReleaseFailed(code) => {
                HttpError::bad_request(format!("Release command failed with exit code {}", code))
            }
            _ => HttpError::internal(anyhow::anyhow!(e)),
        }
    }
}
