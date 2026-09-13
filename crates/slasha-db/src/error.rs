use thiserror::Error;

use crate::crypto;

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
    Pool(#[from] r2d2::Error),

    #[error("query error: {0}")]
    Query(diesel::result::Error),

    #[error("duckdb error: {0}")]
    Duckdb(#[from] duckdb::Error),

    #[error("crypto error: {0}")]
    Crypto(#[from] crypto::CryptoError),

    #[error("task panicked")]
    Join(#[from] tokio::task::JoinError),
}

pub type DbResult<T> = std::result::Result<T, DbError>;

impl From<diesel::result::Error> for DbError {
    fn from(err: diesel::result::Error) -> Self {
        if let diesel::result::Error::DatabaseError(
            diesel::result::DatabaseErrorKind::UniqueViolation,
            info,
        ) = &err
        {
            return DbError::Conflict(info.message().to_string());
        }
        DbError::Query(err)
    }
}
