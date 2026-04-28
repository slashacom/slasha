#[cfg(feature = "bundle")]
pub mod assets;
pub mod auth;
pub mod docker;
pub mod error;
pub mod extractors;
pub mod middleware;
pub mod proxy;
pub mod routing;
pub mod ssh;
pub mod state;
pub mod utils;

pub use error::{Error, Result};
pub use state::AppState;

use diesel_migrations::{EmbeddedMigrations, MigrationHarness, embed_migrations};
use dotenv::dotenv;
use std::net::SocketAddr;
use tokio::net::TcpListener;
use tracing::info;
use tracing_subscriber::{layer::SubscriberExt, util::SubscriberInitExt};

use crate::proxy::container::ensure_caddy_ready;
use crate::state::{Clients, Config, Runtime, Storage};

pub const MIGRATIONS: EmbeddedMigrations = embed_migrations!("../slasha-models/migrations");

fn setup_tracing() {
    tracing_subscriber::registry()
        .with(
            tracing_subscriber::EnvFilter::try_from_default_env()
                .unwrap_or_else(|_| "debug".into()),
        )
        .with(tracing_subscriber::fmt::layer())
        .init();
}

fn setup_dirs() -> (std::path::PathBuf, std::path::PathBuf, std::path::PathBuf) {
    let data_dir = dirs::home_dir()
        .expect("Failed to get home directory")
        .join(".slasha");

    let db_path = utils::ensure_dir(&data_dir).join("slasha.db");
    let repos_dir = utils::ensure_dir(data_dir.join("repos"));
    let logs_dir = utils::ensure_dir(data_dir.join("logs"));

    (db_path, repos_dir, logs_dir)
}

fn run_migrations(storage: &Storage) {
    let mut conn = storage
        .db_pool
        .get()
        .expect("Failed to get DB connection from pool");

    conn.run_pending_migrations(MIGRATIONS)
        .expect("Failed to run migrations");
}

pub async fn run_server(address: Option<SocketAddr>, state: AppState) -> anyhow::Result<()> {
    let address = address.unwrap_or_else(|| "0.0.0.0:3000".parse().unwrap());
    info!("🚀 Slasha server starting on http://{}", address);

    let app = routing::router(state.clone()).with_state(state);
    let listener = TcpListener::bind(address).await?;
    axum::serve(listener, app).await?;

    Ok(())
}

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    dotenv().ok();
    setup_tracing();

    let (db_path, repos_dir, logs_dir) = setup_dirs();

    let config = Config::new(
        std::env::var("JWT_SECRET").expect("JWT_SECRET must be set"),
        std::env::var("SLASHA_PLATFORM_DOMAIN").ok(),
        logs_dir.clone(),
    );

    let docker =
        bollard::Docker::connect_with_local_defaults().expect("Failed to connect to Docker daemon");

    ensure_caddy_ready(&docker).await?;

    let clients = Clients::new(docker.clone());
    let storage = Storage::new(&db_path, repos_dir);

    run_migrations(&storage);

    let proxy_reconcile = proxy::spawn_reconciler(clients.clone(), config.clone());
    let runtime = Runtime::new(4000, 5000, &docker, &logs_dir, proxy_reconcile).await?;
    let state = AppState::new(config, clients, storage, runtime);

    state.runtime.proxy_reconcile.notify_one();

    run_server(Some("0.0.0.0:3000".parse().unwrap()), state).await?;

    Ok(())
}
