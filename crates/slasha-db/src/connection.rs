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
        // Wait for another database writer to finish instead of failing immediately.
        // SQLite only allows one writer at a time, so startup writes may briefly wait.
        diesel::sql_query("PRAGMA busy_timeout=5000;").execute(conn)?;

        diesel::sql_query("PRAGMA journal_mode=WAL;").execute(conn)?;

        // Enforce foreign keys so ON DELETE CASCADE actually fires. Migrations use a
        // separate connection that turns it off (see crate::migrations::run_migrations).
        diesel::sql_query("PRAGMA foreign_keys=ON;").execute(conn)?;

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
