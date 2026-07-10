pub mod alerts;
#[cfg(feature = "bundle")]
pub mod assets;
pub mod auth;
pub mod connections;
pub mod cron;
pub mod docker;
pub mod domain_health;
pub mod extractors;
pub mod logs;
pub mod metrics;
pub mod middleware;
pub mod node_connection_manager;
pub mod proxy;

pub mod routing;

pub mod ssh;
pub mod state;
pub mod tunnel;
pub mod utils;

use std::net::SocketAddr;

use dotenv::dotenv;
pub use routing::api::{HttpError, HttpResult};
use slasha_db::repos::github_app_config::GithubAppConfigRepo;
pub use state::AppState;
use tokio::net::TcpListener;
use tracing::info;

use crate::state::{Clients, Config, Env, Runtime, Storage};

fn setup_tracing() {
    tracing_subscriber::fmt()
        .with_env_filter(
            tracing_subscriber::EnvFilter::try_from_default_env().unwrap_or_else(|_| "info".into()),
        )
        .init();
}

fn setup_dirs() -> (
    std::path::PathBuf, // sqlite db
    std::path::PathBuf, // duckdb
    std::path::PathBuf, // repos
    std::path::PathBuf, // logs
    std::path::PathBuf, // node-ssh-keys
) {
    let data_dir = utils::ensure_dir(
        dirs::home_dir()
            .expect("Failed to get home directory")
            .join(".slasha"),
    );

    (
        data_dir.join("slasha.db"),
        data_dir.join("slasha.duckdb"),
        data_dir.join("repos"),
        data_dir.join("logs"),
        utils::ensure_dir(data_dir.join("node-ssh-keys")),
    )
}

async fn run_server(address: SocketAddr, state: AppState) -> anyhow::Result<()> {
    info!("server starting on http://{}", address);

    let app = routing::router(state.clone()).with_state(state);
    let listener = TcpListener::bind(address).await?;
    axum::serve(listener, app).await?;

    Ok(())
}

pub async fn serve() -> anyhow::Result<()> {
    dotenv().ok();
    setup_tracing();

    let (db_path, duckdb_path, repos_dir, logs_dir, node_ssh_keys_dir) = setup_dirs();

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

    slasha_db::migrations::run_migrations(
        db_path.to_str().expect("Invalid DB path"),
        duckdb_path.to_str().expect("Invalid DuckDB path"),
    );

    let storage = Storage::new(&db_path, &duckdb_path, repos_dir)?;

    let github_config = GithubAppConfigRepo::get(&storage.db_pool).await?;
    let github_client = github_config
        .as_ref()
        .map(connections::GithubClient::from_config)
        .transpose()?;

    let clients = Clients::new(github_client, node_ssh_keys_dir);

    let docker_client = clients
        .docker_registry
        .get_local_client()
        .expect("Failed to connect to local Docker daemon");

    proxy::container::ensure_caddy_ready(&docker_client).await?;
    metrics::app::AppMetricsCollector::new(
        storage.duckdb_pool.clone(),
        storage.db_pool.clone(),
        clients.docker_registry.clone(),
    )
    .spawn();
    metrics::server::ServerMetricsCollector::new(
        storage.duckdb_pool.clone(),
        storage.db_pool.clone(),
        clients.node_connection_manager.clone(),
    )
    .spawn();
    alerts::spawn_alert_worker(
        storage.db_pool.clone(),
        storage.duckdb_pool.clone(),
        config.clone(),
    );

    let proxy_sync_trigger =
        proxy::spawn_route_syncer(clients.clone(), storage.db_pool.clone(), config.clone());
    let runtime = Runtime::new(&logs_dir, proxy_sync_trigger).await?;

    cron::spawn_cron_scheduler(
        storage.db_pool.clone(),
        clients.docker_registry.clone(),
        runtime.log_manager.clone(),
    );

    let state = AppState::new(config, clients, storage, runtime);

    docker::sync::startup_container_sync(
        &state.clients.docker_registry,
        &state.storage.db_pool,
        &state.runtime,
    )
    .await?;

    state.runtime.proxy_sync_trigger.notify_one();

    run_server(SocketAddr::from(([0, 0, 0, 0], port)), state).await?;

    Ok(())
}
