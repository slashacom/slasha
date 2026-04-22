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

    #[error("DB Error: {0}")]
    DBError(#[from] diesel::result::Error),

    #[error("Bad Request: {0}")]
    BadRequest(String),

    #[error("Forbidden: {0}")]
    Forbidden(String),

    #[error("Git Error: {0}")]
    GitError(#[from] GitError),

    #[error("IO Error: {0}")]
    IOError(#[from] std::io::Error),

    #[error("Deployment error: {0}")]
    Deployment(#[from] DeploymentError),
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
            Error::DBError(e) => (StatusCode::INTERNAL_SERVER_ERROR, e.to_string()),
            Error::BadRequest(msg) => (StatusCode::BAD_REQUEST, msg),
            Error::Forbidden(msg) => (StatusCode::FORBIDDEN, msg),
            Error::GitError(_) => unreachable!(),
            Error::IOError(e) => (StatusCode::INTERNAL_SERVER_ERROR, e.to_string()),
            Error::Deployment(e) => {
                tracing::error!("Deployment pipeline error: {:?}", e);
                (
                    StatusCode::INTERNAL_SERVER_ERROR,
                    "Deployment failed".to_string(),
                )
            }
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
