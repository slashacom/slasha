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

    #[error(
        "host port {0} is already in use by another process; slasha needs ports 80, 443 and 2019 free for its Caddy proxy (find the process with `sudo ss -ltnp | grep :{0}`)"
    )]
    PortConflict(u16),

    #[error("Timeout: {0}")]
    Timeout(String),
}
