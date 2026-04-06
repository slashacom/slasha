#[cfg(feature = "bundle")]
pub mod assets;

pub mod auth;
pub mod error;
pub mod extractors;
pub mod middleware;
pub mod routing;
pub mod utils;

use std::net::SocketAddr;
use std::path::PathBuf;
use tokio::net::TcpListener;
use tracing::info;

pub use error::{Error, Result};

use diesel::r2d2::{ConnectionManager, Pool};
use diesel::sqlite::SqliteConnection;

pub type DbPool = Pool<ConnectionManager<SqliteConnection>>;

#[derive(Clone)]
pub struct AppState {
    pub db_pool: DbPool,
    pub jwt_secret: String,
    pub repos_dir: PathBuf,
}

pub async fn run(address: Option<SocketAddr>, state: AppState) -> anyhow::Result<()> {
    let app = routing::router(state.clone()).with_state(state);

    let address = address.unwrap_or_else(|| "0.0.0.0:3000".parse().unwrap());

    info!("🚀 Slasha server starting on http://{}", address);

    let listener = TcpListener::bind(address).await?;
    axum::serve(listener, app).await?;

    Ok(())
}
