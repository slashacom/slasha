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
}

impl IntoResponse for Error {
    fn into_response(self) -> Response {
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
        };

        let body = Json(json!({
            "error": error_message,
        }));

        (status, body).into_response()
    }
}

pub type Result<T> = std::result::Result<T, Error>;
