use std::borrow::Cow;

use diesel::{Connection, RunQueryDsl, sqlite::SqliteConnection};
use diesel_migrations::{EmbeddedMigrations, MigrationHarness, embed_migrations};
use rust_embed::RustEmbed;
use tracing::info;

pub const SQLITE_MIGRATIONS: EmbeddedMigrations = embed_migrations!("migrations/sqlite");

#[derive(RustEmbed)]
#[folder = "migrations/duckdb"]
struct DuckDbMigrations;

fn connect_for_migrations(sqlite_db_path: &str) -> SqliteConnection {
    let mut conn = SqliteConnection::establish(sqlite_db_path)
        .expect("Failed to connect to SQLite for migrations");

    // Can't be left to SQLite's default: vendored builds link libsqlite3-sys with
    // -DSQLITE_DEFAULT_FOREIGN_KEYS=1, and enforced migrations cascade-delete on
    // table rebuilds and reject ADD COLUMN ... REFERENCES on non-empty tables.
    diesel::sql_query("PRAGMA foreign_keys=OFF;")
        .execute(&mut conn)
        .expect("Failed to disable SQLite foreign keys for migrations");

    // Wait out a competing writer rather than panicking the whole boot with
    // "database is locked" if the runtime pool is already touching the file.
    diesel::sql_query("PRAGMA busy_timeout=5000;")
        .execute(&mut conn)
        .expect("Failed to set SQLite busy_timeout for migrations");

    conn
}

pub fn run_migrations(sqlite_db_path: &str, duckdb_path: &str) {
    info!("Running SQLite migrations...");
    let mut conn = connect_for_migrations(sqlite_db_path);

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

#[cfg(test)]
mod tests {
    use diesel::{QueryableByName, sql_types::BigInt};

    use super::*;

    #[derive(QueryableByName)]
    struct Count {
        #[diesel(sql_type = BigInt)]
        n: i64,
    }

    fn count(conn: &mut SqliteConnection, query: &str) -> i64 {
        diesel::sql_query(query)
            .load::<Count>(conn)
            .unwrap_or_else(|e| panic!("Failed to run {query}: {e:?}"))
            .first()
            .map(|row| row.n)
            .unwrap_or_default()
    }

    const SEEDS: &[&str] = &[
        "INSERT OR IGNORE INTO apps (id, slug, name, repo_path)
         VALUES ('app-1', 'demo', 'Demo', '/tmp/demo');",
        "INSERT OR IGNORE INTO deployments (id, app_id, commit_sha, commit_message, status)
         VALUES ('dep-1', 'app-1', 'abc123', 'seed', 'running');",
    ];

    // Failures are expected: a table is only seedable once its migration has run.
    fn seed(conn: &mut SqliteConnection) {
        for statement in SEEDS {
            let _ = diesel::sql_query(*statement).execute(conn);
        }
    }

    // SQLite only rejects ADD COLUMN ... REFERENCES on a table that already has
    // rows, so migrations must be stepped through with data present to catch it.
    #[test]
    fn migrations_apply_to_a_populated_database() {
        let dir = tempfile::tempdir().expect("Failed to create temp dir");
        let db_path = dir.path().join("test.db");
        let db_path = db_path.to_str().expect("Invalid temp path");

        let mut conn = connect_for_migrations(db_path);

        let pending = conn
            .pending_migrations(SQLITE_MIGRATIONS)
            .expect("Failed to collect pending migrations");

        assert!(!pending.is_empty(), "expected migrations to run");

        for migration in pending {
            conn.run_migration(&migration).unwrap_or_else(|e| {
                panic!(
                    "migration {} failed against a populated database: {:?}",
                    migration.name(),
                    e
                )
            });

            seed(&mut conn);
        }

        assert_eq!(
            count(&mut conn, "SELECT count(*) AS n FROM apps;"),
            1,
            "seed rows were never inserted"
        );
        assert_eq!(
            count(&mut conn, "SELECT count(*) AS n FROM deployments;"),
            1,
            "seed rows were never inserted"
        );

        diesel::sql_query("PRAGMA foreign_keys=ON;")
            .execute(&mut conn)
            .expect("Failed to enable foreign keys");

        assert_eq!(
            count(&mut conn, "SELECT count(*) AS n FROM pragma_foreign_key_check;"),
            0,
            "migrations left foreign key violations"
        );
    }
}
