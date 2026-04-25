use axum::{
    Json,
    http::StatusCode,
    response::{IntoResponse, Response},
};
use serde_json::json;
use thiserror::Error;

#[derive(Error, Debug)]
pub enum Error {
    #[error("Internal Server Error: {0}")]
    Internal(#[from] anyhow::Error),

    #[error("Not Found: {0}")]
    NotFound(String),

    #[error("Unauthorized")]
    Unauthorized,

    #[error("Bad Request: {0}")]
    BadRequest(String),

    #[error("Forbidden: {0}")]
    Forbidden(String),

    #[error("Git Error: {0}")]
    GitError(#[from] GitError),

    #[error("Deployment error: {0}")]
    Deployment(#[from] DeploymentError),
}

impl From<diesel::result::Error> for Error {
    fn from(e: diesel::result::Error) -> Self {
        Error::Internal(anyhow::anyhow!(e))
    }
}

impl From<diesel::r2d2::PoolError> for Error {
    fn from(e: diesel::r2d2::PoolError) -> Self {
        Error::Internal(anyhow::anyhow!(e))
    }
}

impl From<std::io::Error> for Error {
    fn from(e: std::io::Error) -> Self {
        Error::Internal(anyhow::anyhow!(e))
    }
}

#[derive(Error, Debug)]
pub enum GitError {
    #[error("Unauthorized")]
    Unauthorized,
    #[error("Bad Request: {0}")]
    BadRequest(String),
    #[error("Repository Not Found")]
    RepoNotFound,
    #[error("Invalid Credentials")]
    InvalidCredentials,
    #[error("Not a member")]
    NotMember,
    #[error("Internal Server Error: {0}")]
    Internal(#[from] anyhow::Error),
}

#[derive(Debug, Error)]
pub enum DeploymentError {
    #[error("DB pool error: {0}")]
    PoolError(#[from] diesel::r2d2::PoolError),

    #[error("Database error: {0}")]
    DatabaseError(#[from] diesel::result::Error),

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

    #[error("Temp directory error: {0}")]
    TempDir(std::io::Error),

    #[error("Path is not valid UTF-8")]
    PathNotUtf8,

    #[error("Proxy error: {0}")]
    Proxy(#[from] crate::error::ProxyError),
}

#[derive(Debug, Error)]
pub enum ProxyError {
    #[error("Docker API error: {0}")]
    DockerApi(#[from] bollard::errors::Error),

    #[error("Database error: {0}")]
    DatabaseError(#[from] diesel::result::Error),

    #[error("DB pool error: {0}")]
    PoolError(#[from] diesel::r2d2::PoolError),

    #[error("HTTP error: {0}")]
    Http(#[from] reqwest::Error),

    #[error("Caddy error: {0}")]
    Caddy(String),

    #[error("Timeout: {0}")]
    Timeout(String),
}

impl IntoResponse for Error {
    fn into_response(self) -> Response {
        if let Error::GitError(e) = self {
            return e.into_response();
        }

        let (status, message) = match self {
            Error::Internal(e) => {
                tracing::error!("Internal server error: {:?}", e);
                (
                    StatusCode::INTERNAL_SERVER_ERROR,
                    "Internal Server Error".to_string(),
                )
            }
            Error::NotFound(msg) => (StatusCode::NOT_FOUND, msg),
            Error::Unauthorized => (StatusCode::UNAUTHORIZED, "Unauthorized".to_string()),
            Error::BadRequest(msg) => (StatusCode::BAD_REQUEST, msg),
            Error::Forbidden(msg) => (StatusCode::FORBIDDEN, msg),
            Error::Deployment(e) => match e {
                DeploymentError::ServiceNotFound(name) => (StatusCode::NOT_FOUND, name),
                DeploymentError::ServiceNotRunning(name) => (
                    StatusCode::BAD_REQUEST,
                    format!("Service {} is not running", name),
                ),
                DeploymentError::KeyNotExported(svc, key) => (
                    StatusCode::BAD_REQUEST,
                    format!("Service {} does not export key {}", svc, key),
                ),
                _ => {
                    tracing::error!("Deployment pipeline error: {:?}", e);
                    (
                        StatusCode::INTERNAL_SERVER_ERROR,
                        "Deployment failed".to_string(),
                    )
                }
            },

            _ => unreachable!(),
        };

        let body = Json(json!({ "error": message }));
        (status, body).into_response()
    }
}

impl IntoResponse for GitError {
    fn into_response(self) -> Response {
        match self {
            GitError::Unauthorized => (
                StatusCode::UNAUTHORIZED,
                [(axum::http::header::WWW_AUTHENTICATE, "Basic realm=\"Git\"")],
                "Unauthorized",
            )
                .into_response(),
            GitError::RepoNotFound => {
                (StatusCode::NOT_FOUND, "Repository Not Found").into_response()
            }
            GitError::InvalidCredentials => {
                (StatusCode::UNAUTHORIZED, "Invalid Credentials").into_response()
            }
            GitError::NotMember => (StatusCode::FORBIDDEN, "Not a member").into_response(),
            GitError::BadRequest(msg) => (StatusCode::BAD_REQUEST, msg).into_response(),
            GitError::Internal(msg) => {
                (StatusCode::INTERNAL_SERVER_ERROR, msg.to_string()).into_response()
            }
        }
    }
}

pub type Result<T> = std::result::Result<T, Error>;
