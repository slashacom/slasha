use diesel::{
    RunQueryDsl,
    r2d2::{self, ConnectionManager, CustomizeConnection, Error},
    sqlite::SqliteConnection,
};

use crate::error::{DbError, DbResult};

pub type DbPool = r2d2::Pool<ConnectionManager<SqliteConnection>>;
pub type DuckdbPool = r2d2::Pool<duckdb::DuckdbConnectionManager>;

#[derive(Debug)]
struct SqliteConnectionCustomizer;

impl CustomizeConnection<SqliteConnection, Error> for SqliteConnectionCustomizer {
    fn on_acquire(&self, conn: &mut SqliteConnection) -> Result<(), Error> {
        // Wait for a competing writer to release the lock instead of failing
        // immediately with "database is locked". SQLite serialises writers even
        // in WAL mode, so concurrent writers (e.g. the metrics collectors at
        // startup) need this to ride out contention.
        diesel::sql_query("PRAGMA busy_timeout=5000;")
            .execute(conn)
            .map_err(Error::QueryError)?;

        diesel::sql_query("PRAGMA journal_mode=WAL;")
            .execute(conn)
            .map_err(Error::QueryError)?;

        // Enforce foreign keys so ON DELETE CASCADE actually fires; SQLite leaves
        // this off per connection by default. Migrations use a separate, un-enforced
        // connection (see run_migrations) so table rebuilds don't cascade-delete.
        diesel::sql_query("PRAGMA foreign_keys=ON;")
            .execute(conn)
            .map_err(Error::QueryError)?;

        Ok(())
    }
}

pub fn create_pool_with_max_size(db_path: &str, max_size: u32) -> DbResult<DbPool> {
    let manager = ConnectionManager::new(db_path);

    r2d2::Pool::builder()
        .max_size(max_size)
        .connection_customizer(Box::new(SqliteConnectionCustomizer))
        .build(manager)
        .map_err(DbError::Pool)
}

pub fn create_duckdb_pool_with_max_size(db_path: &str, max_size: u32) -> DbResult<DuckdbPool> {
    let manager = duckdb::DuckdbConnectionManager::file(db_path)?;

    r2d2::Pool::builder()
        .max_size(max_size)
        .build(manager)
        .map_err(DbError::Pool)
}
