pub mod assets;
pub mod error;
pub mod routing;

use std::net::SocketAddr;
use tokio::net::TcpListener;
use tracing::info;

pub use error::{Error, Result};

pub async fn run(address: Option<SocketAddr>) -> anyhow::Result<()> {
    let app = routing::router();

    let address = address.unwrap_or_else(|| "0.0.0.0:3000".parse().unwrap());

    info!("🚀 Slasha server starting on http://{}", address);

    let listener = TcpListener::bind(address).await?;
    axum::serve(listener, app).await?;

    Ok(())
}
