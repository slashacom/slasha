pub mod connection;
pub mod crypto;
pub mod error;
pub mod migrations;
pub mod models;
pub mod repos;

pub use connection::{
    DbPool, DuckdbPool, create_duckdb_pool_with_max_size, create_pool_with_max_size,
};
pub use error::{DbError, DbResult};
pub use models::{
    alerts, app, app_backup, app_metrics, cron, deployment, git_connection, github_app_config,
    github_connection, logs, node, node_metrics, schema, service, ssh_keys, user,
};

/// Initializes the database by configuring encryption and applying pending migrations.
pub fn init(sqlite_db_path: &str, duckdb_path: &str, secret_key: Option<&str>) {
    crypto::init(secret_key);
    migrations::run_migrations(sqlite_db_path, duckdb_path);
}
