use diesel::{
    r2d2::{ConnectionManager, CustomizeConnection, Pool},
    sqlite::SqliteConnection,
};

use crate::error::{DbError, DbResult};

pub type DbPool = Pool<ConnectionManager<SqliteConnection>>;

#[derive(Debug)]
struct WalCustomizer;

impl CustomizeConnection<SqliteConnection, diesel::r2d2::Error> for WalCustomizer {
    fn on_acquire(&self, conn: &mut SqliteConnection) -> Result<(), diesel::r2d2::Error> {
        use diesel::RunQueryDsl;
        diesel::sql_query("PRAGMA journal_mode=WAL;")
            .execute(conn)
            .map_err(diesel::r2d2::Error::QueryError)?;
        Ok(())
    }
}

pub fn create_pool(db_path: &str) -> DbResult<DbPool> {
    create_pool_with_max_size(db_path, 10)
}

pub fn create_pool_with_max_size(db_path: &str, max_size: u32) -> DbResult<DbPool> {
    let manager = ConnectionManager::<SqliteConnection>::new(db_path);
    Pool::builder()
        .max_size(max_size)
        .connection_customizer(Box::new(WalCustomizer))
        .build(manager)
        .map_err(DbError::Pool)
}
