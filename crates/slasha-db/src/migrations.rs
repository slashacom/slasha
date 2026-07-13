use std::borrow::Cow;

use diesel::{Connection, sqlite::SqliteConnection};
use diesel_migrations::{EmbeddedMigrations, MigrationHarness, embed_migrations};
use rust_embed::RustEmbed;
use tracing::info;

pub const SQLITE_MIGRATIONS: EmbeddedMigrations = embed_migrations!("migrations/sqlite");

#[derive(RustEmbed)]
#[folder = "migrations/duckdb"]
struct DuckDbMigrations;

pub fn run_migrations(sqlite_db_path: &str, duckdb_path: &str) {
    info!("Running SQLite migrations...");
    // migrations run on a dedicated connection that leaves foreign keys at SQLite's
    // default (off). The runtime pool enforces them, but a table-rebuild migration
    // under enforcement would cascade-delete rows, so migrations must not enforce.
    let mut conn = SqliteConnection::establish(sqlite_db_path)
        .expect("Failed to connect to SQLite for migrations");

    let sqlite_pending = conn
        .pending_migrations(SQLITE_MIGRATIONS)
        .expect("Failed to check pending SQLite migrations")
        .len();

    if sqlite_pending > 0 {
        conn.run_pending_migrations(SQLITE_MIGRATIONS)
            .expect("Failed to run SQLite migrations");

        info!("Applied {} SQLite migrations successfully", sqlite_pending);
    } else {
        info!("No SQLite migrations to apply");
    }

    info!("Running DuckDB migrations...");
    let duckdb_conn =
        duckdb::Connection::open(duckdb_path).expect("Failed to connect to DuckDB for migrations");
    duckdb_conn
        .execute(
            "CREATE TABLE IF NOT EXISTS __duckdb_migrations (
            id VARCHAR PRIMARY KEY,
            applied_at TIMESTAMP DEFAULT CURRENT_TIMESTAMP
        )",
            [],
        )
        .expect("Failed to create __duckdb_migrations table");

    let mut stmt = duckdb_conn
        .prepare("SELECT id FROM __duckdb_migrations")
        .unwrap();

    let applied_migrations: Vec<String> = stmt
        .query_map([], |row| row.get(0))
        .unwrap()
        .map(|res| res.unwrap())
        .collect();

    let mut files: Vec<Cow<'static, str>> = DuckDbMigrations::iter().collect();
    files.sort();

    let mut duckdb_pending = 0;

    for file in files {
        let migration_id = file.as_ref();

        if !applied_migrations.contains(&migration_id.to_string()) {
            duckdb_pending += 1;

            info!("Applying DuckDB migration: {}", migration_id);

            let file = DuckDbMigrations::get(migration_id).unwrap();
            let sql = std::str::from_utf8(file.data.as_ref()).unwrap();

            duckdb_conn.execute_batch(sql).unwrap_or_else(|e| {
                panic!("Failed to execute migration {}: {:?}", migration_id, e)
            });

            duckdb_conn
                .execute(
                    "INSERT INTO __duckdb_migrations (id) VALUES (?)",
                    [migration_id],
                )
                .unwrap();
        }
    }

    if duckdb_pending > 0 {
        info!("Applied {} DuckDB migrations successfully", duckdb_pending);
    } else {
        info!("No DuckDB migrations to apply");
    }
}
