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
