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
    let manager = ConnectionManager::<SqliteConnection>::new(db_path);
    Pool::builder()
        .connection_customizer(Box::new(WalCustomizer))
        .build(manager)
        .map_err(DbError::Pool)
}
