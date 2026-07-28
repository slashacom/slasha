use thiserror::Error;

use crate::{HttpError, operations::OperationError, proxy::ProxyError};

#[derive(Debug, Error)]
pub enum DockerError {
    #[error("Database error: {0}")]
    Db(#[from] slasha_db::DbError),

    #[error("{0}")]
    Operation(#[from] OperationError),

    #[error("Git error: {0}")]
    Git(#[from] git2::Error),

    #[error("Git archive failed: {0}")]
    GitArchiveFailed(String),

    #[error("Docker client error: {0}")]
    DockerClient(#[from] bollard::errors::Error),

    #[error("Build failed: {0}")]
    BuildFailed(String),

    #[error("Service \"{0}\" not found")]
    ServiceNotFound(String),

    #[error("Service \"{0}\" is not running")]
    ServiceNotRunning(String),

    #[error("Environment resolution failed: {0}")]
    EnvResolveFailed(String),

    #[error("I/O error: {0}")]
    Io(#[from] std::io::Error),

    #[error("UTF-8 error: {0}")]
    Utf8(#[from] std::str::Utf8Error),

    #[error("Tokio join error: {0}")]
    TokioJoin(#[from] tokio::task::JoinError),

    #[error("Proxy error: {0}")]
    Proxy(#[from] ProxyError),

    #[error("Scale error: {0}")]
    ScaleError(String),

    #[error("Release command failed with exit code {0}")]
    ReleaseFailed(i64),

    #[error("Service \"{0}\" did not become healthy within {1}s")]
    ServiceHealthcheckTimeout(String, u64),

    #[error("Service \"{0}\" reported unhealthy")]
    ServiceHealthcheckFailed(String),

    #[error("App failed readiness check: {0}")]
    AppNotReady(String),

    #[error("Deployment \"{0}\" no longer has a retained image")]
    ArtifactUnavailable(String),

    #[error("Validation error: {0}")]
    Validation(String),

    #[error("{0}")]
    Other(#[from] anyhow::Error),
}

pub type DockerResult<T> = std::result::Result<T, DockerError>;

impl From<DockerError> for HttpError {
    fn from(e: DockerError) -> Self {
        match e {
            DockerError::ServiceNotFound(msg) => HttpError::not_found(msg),
            DockerError::ServiceNotRunning(msg) => {
                HttpError::bad_request(format!("Service {} is not running", msg))
            }
            DockerError::ReleaseFailed(code) => {
                HttpError::bad_request(format!("Release command failed with exit code {}", code))
            }
            DockerError::ArtifactUnavailable(msg) => {
                HttpError::bad_request(format!("Deployment {} no longer has a retained image", msg))
            }
            DockerError::Validation(msg) => HttpError::bad_request(msg),
            DockerError::EnvResolveFailed(msg) => HttpError::bad_request(msg),
            DockerError::Git(err) => HttpError::bad_request(err.message()),
            DockerError::Operation(err) => HttpError::bad_request(err.to_string()),
            _ => HttpError::internal(e),
        }
    }
}
