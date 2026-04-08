#[cfg(feature = "bundle")]
pub mod assets;
pub mod auth;
pub mod error;
pub mod extractors;
pub mod middleware;
pub mod routing;
pub mod ssh;
pub mod utils;

use std::net::SocketAddr;
use std::path::PathBuf;
use tokio::net::TcpListener;
use tracing::info;

pub use error::{Error, Result};

use diesel::r2d2::{ConnectionManager, Pool};
use diesel::sqlite::SqliteConnection;
use diesel_migrations::{EmbeddedMigrations, MigrationHarness, embed_migrations};
use dotenv::dotenv;
use tracing_subscriber::{layer::SubscriberExt, util::SubscriberInitExt};

#[derive(Clone)]
pub struct AppState {
    pub db_pool: Pool<ConnectionManager<SqliteConnection>>,
    pub jwt_secret: String,
    pub repos_dir: PathBuf,
}

pub const MIGRATIONS: EmbeddedMigrations = embed_migrations!("../models/migrations");

fn run_migrations(state: &AppState) -> anyhow::Result<()> {
    let mut conn = state
        .db_pool
        .get()
        .expect("Failed to get DB connection from pool");
    conn.run_pending_migrations(MIGRATIONS)
        .expect("Failed to run migrations");
    Ok(())
}

pub async fn run_server(address: Option<SocketAddr>, state: AppState) -> anyhow::Result<()> {
    let app = routing::router(state.clone()).with_state(state);

    let address = address.unwrap_or_else(|| "0.0.0.0:3000".parse().unwrap());

    info!("🚀 Slasha server starting on http://{}", address);

    let listener = TcpListener::bind(address).await?;
    axum::serve(listener, app).await?;

    Ok(())
}

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    dotenv().ok();

    tracing_subscriber::registry()
        .with(
            tracing_subscriber::EnvFilter::try_from_default_env()
                .unwrap_or_else(|_| "debug".into()),
        )
        .with(tracing_subscriber::fmt::layer())
        .init();

    let data_dir = dirs::home_dir()
        .expect("Failed to get home directory")
        .join(".slasha");

    let db_path = utils::ensure_dir(&data_dir).join("slasha.db");
    let repos_dir = utils::ensure_dir(&data_dir.join("repos"));

    let state = AppState {
        db_pool: Pool::builder()
            .build(ConnectionManager::<SqliteConnection>::new(
                db_path.to_str().unwrap(),
            ))
            .expect("Failed to create DB pool"),
        jwt_secret: std::env::var("JWT_SECRET").expect("JWT_SECRET must be set"),
        repos_dir,
    };

    run_migrations(&state)?;

    run_server(Some("0.0.0.0:3000".parse().unwrap()), state).await?;

    Ok(())
}
