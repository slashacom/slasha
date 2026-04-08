pub mod apps;
pub mod auth;
mod clap_app;
pub mod config;
pub mod http;

use crate::{
    clap_app::{AppsCommand, ClapApp, Command},
    config::Config,
};
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
        Command::Login => {
            auth::handle_login().await?;
        }

        Command::Me => {
            auth::handle_me().await?;
        }
        Command::SetUrl { url } => {
            let mut conf = Config::load()?;
            conf.base_url = url.clone();
            conf.save()?;

            tracing::info!("Set Slasha API URL to: {}", url);
        }
        Command::Status => {
            let res = http::client()?.get("/api/health").await;

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
                Err(e) => {
                    tracing::error!("Could not connect to Slasha server: {}", e);
                }
            }
        }
        Command::Apps { command } => match command {
            AppsCommand::Create { name } => {
                apps::handle_create(&name).await?;
            }
            AppsCommand::Delete { slug } => {
                apps::handle_delete(&slug).await?;
            }
            AppsCommand::Info { slug } => {
                apps::handle_info(&slug).await?;
            }
            AppsCommand::List => {
                apps::handle_list().await?;
            }
        },
    }

    Ok(())
}
