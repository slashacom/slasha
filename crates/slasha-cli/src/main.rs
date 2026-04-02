mod clap_app;

use crate::clap_app::{ClapApp, Command};
use clap::Parser;
use tracing_subscriber::{layer::SubscriberExt, util::SubscriberInitExt};

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    tracing_subscriber::registry()
        .with(
            tracing_subscriber::EnvFilter::try_from_default_env().unwrap_or_else(|_| "info".into()),
        )
        .with(tracing_subscriber::fmt::layer())
        .init();

    let cli = ClapApp::parse();

    match cli.command {
        Command::Status => {
            let client = reqwest::Client::new();
            let res = client.get("http://localhost:3000/api/health").send().await;

            match res {
                Ok(response) => {
                    if response.status().is_success() {
                        let body: serde_json::Value = response.json().await?;
                        tracing::info!("Status: {}", body["status"]);
                        tracing::info!("Version: {}", body["version"]);
                    } else {
                        tracing::error!("Slasha server returned error: {}", response.status());
                    }
                }
                Err(_) => {
                    tracing::error!("Could not connect to Slasha server at http://localhost:3000");
                }
            }
        }
    }

    Ok(())
}
