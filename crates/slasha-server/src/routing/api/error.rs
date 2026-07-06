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

    pub fn validation(message: String) -> Self {
        Self::new(StatusCode::UNPROCESSABLE_ENTITY, message)
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
            tracing::error!(error = ?cause, "Internal server error");
        }

        let body = json!({ "error": self.message });
        let mut response = (self.status, Json(body)).into_response();

        for (name, value) in self.extra_headers {
            if let Ok(value) = value.parse() {
                response.headers_mut().insert(name, value);
            }
        }

        response
    }
}

impl From<garde::Report> for HttpError {
    fn from(report: garde::Report) -> Self {
        let mut first_error = None;
        for (path, error) in report.into_inner() {
            if first_error.is_none() {
                let error_str = error.to_string();
                let path_str = path.to_string();

                let formatted_path = if path_str.is_empty() {
                    String::new()
                } else {
                    let field = path_str.split('.').next_back().unwrap_or(&path_str);
                    let mut human_readable = field.replace("_", " ");
                    if let Some(r) = human_readable.get_mut(0..1) {
                        r.make_ascii_uppercase();
                    }
                    format!("{human_readable} ")
                };

                first_error = Some(format!("{}{}", formatted_path, error_str));
            }
        }
        let message = first_error.unwrap_or_else(|| "Validation failed".to_string());
        Self::validation(message)
    }
}

impl From<slasha_db::DbError> for HttpError {
    fn from(error: slasha_db::DbError) -> Self {
        match error {
            slasha_db::DbError::NotFound(message) => HttpError::not_found(message),
            slasha_db::DbError::PreconditionFailed(message)
            | slasha_db::DbError::Conflict(message)
            | slasha_db::DbError::Data(message) => HttpError::bad_request(message),
            _ => HttpError::internal(anyhow::anyhow!(error)),
        }
    }
}

impl From<anyhow::Error> for HttpError {
    fn from(error: anyhow::Error) -> Self {
        HttpError::internal(error)
    }
}

impl From<std::io::Error> for HttpError {
    fn from(error: std::io::Error) -> Self {
        HttpError::internal(error)
    }
}

impl From<ProxyError> for HttpError {
    fn from(error: ProxyError) -> Self {
        match error {
            ProxyError::Caddy(message) => HttpError::bad_request(message),
            ProxyError::Timeout(message) => HttpError::new(StatusCode::GATEWAY_TIMEOUT, message),
            _ => HttpError::internal(anyhow::anyhow!(error)),
        }
    }
}

pub type HttpResult<T> = Result<T, HttpError>;
