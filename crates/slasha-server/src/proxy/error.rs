use thiserror::Error;

#[derive(Debug, Error)]
pub enum ProxyError {
    #[error("Docker client error: {0}")]
    DockerApi(#[from] bollard::errors::Error),

    #[error("Database error: {0}")]
    Db(#[from] slasha_db::DbError),

    #[error("HTTP error: {0}")]
    Http(#[from] reqwest::Error),

    #[error("Caddy error: {0}")]
    Caddy(String),

    #[error("Timeout: {0}")]
    Timeout(String),
}

pub type ProxyResult<T> = std::result::Result<T, ProxyError>;
