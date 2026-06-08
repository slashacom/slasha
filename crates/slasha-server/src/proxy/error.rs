use thiserror::Error;

#[derive(Debug, Error)]
pub enum ProxyError {
    #[error("Docker API error: {0}")]
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
