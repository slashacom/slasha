use reqwest::StatusCode;
use thiserror::Error;

use crate::HttpError;

#[derive(Debug, Error)]
pub enum TunnelError {
    #[error("service container is not running")]
    NotRunning,
    #[error("tunnel limit reached ({0} concurrent tunnels per user)")]
    LimitReached(usize),
    #[error("invalid PORT env var: '{0}' is not a valid port number")]
    InvalidPort(String),
    #[error("docker error: {0}")]
    Docker(#[from] bollard::errors::Error),
    #[error("io error: {0}")]
    Io(#[from] std::io::Error),
    #[error("websocket error: {0}")]
    Websocket(#[from] axum::Error),
}

impl From<TunnelError> for HttpError {
    fn from(e: TunnelError) -> Self {
        match e {
            TunnelError::NotRunning => HttpError::bad_request("Service container is not running"),
            TunnelError::LimitReached(n) => HttpError::new(
                StatusCode::TOO_MANY_REQUESTS,
                format!("Tunnel limit reached ({n} concurrent tunnels per user)"),
            ),
            TunnelError::InvalidPort(v) => HttpError::bad_request(format!(
                "Invalid PORT env var: '{v}' is not a valid port number"
            )),
            TunnelError::Docker(e) => HttpError::internal(e),
            TunnelError::Io(e) => HttpError::internal(e),
            TunnelError::Websocket(e) => HttpError::internal(e),
        }
    }
}

pub type TunnelResult<T> = std::result::Result<T, TunnelError>;
