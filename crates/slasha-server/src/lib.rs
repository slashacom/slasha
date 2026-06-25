#[cfg(feature = "bundle")]
pub mod assets;
pub mod auth;
pub mod docker;
pub mod error;
pub mod extractors;
pub mod metrics;
pub mod middleware;
pub mod proxy;

pub mod routing;
pub mod server_metrics;
pub mod ssh;
pub mod state;
pub mod tunnel;
pub mod utils;

use std::net::SocketAddr;

use diesel_migrations::{EmbeddedMigrations, MigrationHarness, embed_migrations};
use dotenv::dotenv;
pub use error::{HttpError, HttpResult};
pub use state::AppState;
use tokio::net::TcpListener;
use tracing::info;

use crate::state::{Clients, Config, Env, Runtime, Storage};

pub const MIGRATIONS: EmbeddedMigrations = embed_migrations!("../slasha-db/migrations");

fn setup_tracing() {
    tracing_subscriber::fmt()
        .with_env_filter(
            tracing_subscriber::EnvFilter::try_from_default_env().unwrap_or_else(|_| "info".into()),
        )
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

pub async fn run_server(address: SocketAddr, state: AppState) -> anyhow::Result<()> {
    info!("server starting on http://{}", address);

    let app = routing::router(state.clone()).with_state(state);
    let listener = TcpListener::bind(address).await?;
    axum::serve(listener, app).await?;

    Ok(())
}

pub async fn serve() -> anyhow::Result<()> {
    dotenv().ok();
    setup_tracing();

    let (db_path, repos_dir, logs_dir) = setup_dirs();

    let slasha_env = Env::from_str_or_default(
        &std::env::var("SLASHA_ENV").unwrap_or_else(|_| "development".to_string()),
    );

    let jwt_secret = std::env::var("JWT_SECRET").expect("JWT_SECRET must be set");
    let platform_domain =
        std::env::var("SLASHA_PLATFORM_DOMAIN").expect("SLASHA_PLATFORM_DOMAIN must be set");
    let port = std::env::var("SLASHA_PORT")
        .ok()
        .and_then(|p| p.parse().ok())
        .unwrap_or(3000);

    let config = Config::new(
        slasha_env,
        jwt_secret,
        platform_domain,
        logs_dir.clone(),
        port,
    );

    let docker_client =
        bollard::Docker::connect_with_local_defaults().expect("Failed to connect to Docker daemon");

    proxy::container::ensure_caddy_ready(&docker_client).await?;

    let clients = Clients::new(docker_client.clone());
    let storage = Storage::new(&db_path, repos_dir)?;

    run_migrations(&storage);

    metrics::spawn_app_metrics_collector(storage.db_pool.clone(), docker_client.clone());
    server_metrics::spawn_server_metrics_collector(storage.db_pool.clone());

    let proxy_sync_trigger =
        proxy::spawn_route_syncer(clients.clone(), storage.db_pool.clone(), config.clone());
    let runtime = Runtime::new(&logs_dir, proxy_sync_trigger).await?;
    let state = AppState::new(config, clients, storage, runtime);

    docker::sync::startup_container_sync(
        &state.clients.docker,
        &state.storage.db_pool,
        &state.runtime,
    )
    .await?;

    state.runtime.proxy_sync_trigger.notify_one();

    run_server(SocketAddr::from(([0, 0, 0, 0], port)), state).await?;

    Ok(())
}
