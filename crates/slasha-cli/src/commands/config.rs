use anyhow::Result;
use colored::Colorize;

use crate::{clap_app::ConfigCommand, config::GlobalConfig, output::cli_success};

pub async fn dispatch(cmd: ConfigCommand) -> Result<()> {
    match cmd {
        ConfigCommand::Set { key, value } => handle_set(&key, &value),
        ConfigCommand::Get { key } => handle_get(&key),
    }
}

fn handle_set(key: &str, value: &str) -> Result<()> {
    let key = normalize_key(key);

    match key {
        "server-url" => {
            let mut config = GlobalConfig::load()?;
            config.server_url = Some(value.to_owned());
            config.save()?;

            cli_success(format!("Set {} to {}", key.cyan(), value.cyan()));

            Ok(())
        }
        _ => anyhow::bail!("Unknown key: {key}"),
    }
}

fn handle_get(key: &str) -> Result<()> {
    let key = normalize_key(key);
    let config = GlobalConfig::load()?;

    match key {
        "server-url" => {
            cli_success(format!(
                "{key} = {}",
                config.server_url.as_deref().unwrap_or("").cyan()
            ));

            Ok(())
        }
        _ => anyhow::bail!("Unknown key: {key}"),
    }
}

fn normalize_key(key: &str) -> &str {
    key.trim()
}
