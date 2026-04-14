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
