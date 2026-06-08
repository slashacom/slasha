use axum::{
    Json,
    http::{HeaderName, StatusCode},
    response::{IntoResponse, Response},
};
use serde_json::json;

use crate::proxy::ProxyError;

pub struct HttpError {
    pub status: StatusCode,
    pub message: String,
    pub cause: Option<Box<dyn std::error::Error + Send + Sync>>,
    pub extra_headers: Vec<(HeaderName, String)>,
}

impl std::fmt::Debug for HttpError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("HttpError")
            .field("status", &self.status)
            .field("message", &self.message)
            .field("cause", &self.cause)
            .finish()
    }
}

impl HttpError {
    pub fn new(status: StatusCode, message: impl Into<String>) -> Self {
        Self {
            status,
            message: message.into(),
            cause: None,
            extra_headers: Vec::new(),
        }
    }

    pub fn internal(e: impl Into<Box<dyn std::error::Error + Send + Sync>>) -> Self {
        Self {
            status: StatusCode::INTERNAL_SERVER_ERROR,
            message: "Internal Server Error".to_string(),
            cause: Some(e.into()),
            extra_headers: Vec::new(),
        }
    }

    pub fn not_found(message: impl Into<String>) -> Self {
        Self::new(StatusCode::NOT_FOUND, message)
    }

    pub fn bad_request(message: impl Into<String>) -> Self {
        Self::new(StatusCode::BAD_REQUEST, message)
    }

    pub fn unauthorized() -> Self {
        Self::new(StatusCode::UNAUTHORIZED, "Unauthorized")
    }

    pub fn forbidden(message: impl Into<String>) -> Self {
        Self::new(StatusCode::FORBIDDEN, message)
    }

    pub fn with_headers(mut self, headers: impl IntoIterator<Item = (HeaderName, String)>) -> Self {
        self.extra_headers.extend(headers);
        self
    }
}

impl IntoResponse for HttpError {
    fn into_response(self) -> Response {
        if self.status.is_server_error()
            && let Some(cause) = &self.cause
        {
            tracing::error!(
                error = ?cause,
                "Internal server error"
            );
        }

        let mut res = (self.status, Json(json!({ "error": self.message }))).into_response();

        for (name, value) in self.extra_headers {
            if let Ok(value) = value.parse() {
                res.headers_mut().insert(name, value);
            }
        }

        res
    }
}

impl From<slasha_db::DbError> for HttpError {
    fn from(e: slasha_db::DbError) -> Self {
        match e {
            slasha_db::DbError::NotFound(msg) => HttpError::not_found(msg),
            slasha_db::DbError::PreconditionFailed(msg) | slasha_db::DbError::Conflict(msg) => {
                HttpError::bad_request(msg)
            }
            _ => HttpError::internal(anyhow::anyhow!(e)),
        }
    }
}

impl From<anyhow::Error> for HttpError {
    fn from(e: anyhow::Error) -> Self {
        HttpError::internal(e)
    }
}

impl From<std::io::Error> for HttpError {
    fn from(e: std::io::Error) -> Self {
        HttpError::internal(e)
    }
}

impl From<ProxyError> for HttpError {
    fn from(e: ProxyError) -> Self {
        match e {
            ProxyError::Caddy(msg) => HttpError::bad_request(msg),
            ProxyError::Timeout(msg) => HttpError::new(StatusCode::GATEWAY_TIMEOUT, msg),
            _ => HttpError::internal(anyhow::anyhow!(e)),
        }
    }
}

pub type HttpResult<T> = std::result::Result<T, HttpError>;
