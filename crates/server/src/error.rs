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

    #[error("Git Error: {0}")]
    GitError(#[from] GitError),
}

#[derive(Error, Debug)]
pub enum GitError {
    #[error("Unauthorized")]
    Unauthorized,
    #[error("Bad Request: {0}")]
    BadRequest(String),
    #[error("Internal Server Error: {0}")]
    Internal(#[from] anyhow::Error),
}

impl IntoResponse for Error {
    fn into_response(self) -> Response {
        match self {
            Error::GitError(e) => e.into_response(),
            _ => {
                let (status, error_message) = match self {
                    Error::Internal(ref e) => {
                        tracing::error!("Internal server error: {:?}", e);
                        (
                            StatusCode::INTERNAL_SERVER_ERROR,
                            "Internal Server Error".to_string(),
                        )
                    }
                    Error::NotFound(ref m) => (StatusCode::NOT_FOUND, m.clone()),
                    Error::Unauthorized => (StatusCode::UNAUTHORIZED, "Unauthorized".to_string()),
                    Error::DBError(ref e) => (StatusCode::INTERNAL_SERVER_ERROR, e.to_string()),
                    Error::BadRequest(ref m) => (StatusCode::BAD_REQUEST, m.clone()),
                    Error::GitError(_) => unreachable!(),
                };

                let body = Json(json!({
                    "error": error_message,
                }));

                (status, body).into_response()
            }
        }
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
            GitError::BadRequest(msg) => (StatusCode::BAD_REQUEST, msg).into_response(),
            GitError::Internal(msg) => (StatusCode::INTERNAL_SERVER_ERROR, msg.to_string()).into_response()
        }
    }
}

pub type Result<T> = std::result::Result<T, Error>;
