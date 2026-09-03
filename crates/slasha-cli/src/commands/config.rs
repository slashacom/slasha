use anyhow::Result;
use colored::Colorize;

use crate::{
    clap_app::ConfigCommand,
    config::GlobalConfig,
    output::{cli_label, cli_success},
};

pub async fn dispatch(cmd: ConfigCommand) -> Result<()> {
    match cmd {
        ConfigCommand::Set { key, value } => handle_set(&key, &value),
        ConfigCommand::Get { key } => handle_get(&key),
    }
}

fn handle_set(key: &str, value: &str) -> Result<()> {
    let key = normalize_key(key);
    let mut config = GlobalConfig::load()?;

    match key.as_str() {
        "server-url" => {
            config.server_url = Some(value.to_owned());
        }
        _ => anyhow::bail!("Unknown key: {key}"),
    }

    config.save()?;
    cli_success(format!("Set {} to {}", key, value.cyan()));

    Ok(())
}

fn handle_get(key: &str) -> Result<()> {
    let key = normalize_key(key);
    let config = GlobalConfig::load()?;

    let value = match key.as_str() {
        "server-url" => config.server_url.as_deref().unwrap_or("(not set)"),
        _ => anyhow::bail!("Unknown key: {key}"),
    };

    cli_label(key, value.cyan());

    Ok(())
}

fn normalize_key(key: &str) -> String {
    key.trim().replace('_', "-").to_lowercase()
}
