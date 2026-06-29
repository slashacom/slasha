use thiserror::Error;

#[derive(Debug, Error)]
pub enum DbError {
    #[error("not found: {0}")]
    NotFound(String),

    #[error("precondition failed: {0}")]
    PreconditionFailed(String),

    #[error("conflict: {0}")]
    Conflict(String),

    #[error("data integrity error: {0}")]
    Data(String),

    #[error("pool error: {0}")]
    Pool(#[from] diesel::r2d2::PoolError),

    #[error("query error: {0}")]
    Query(#[from] diesel::result::Error),

    #[error("task panicked")]
    Join(#[from] tokio::task::JoinError),
}

pub type DbResult<T> = std::result::Result<T, DbError>;
