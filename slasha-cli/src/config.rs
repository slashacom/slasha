use std::{fs, path::PathBuf};

use anyhow::{Context, Result};
use serde::{Deserialize, Serialize};

const CONFIG_PATH: &str = "slasha.toml";

#[derive(Debug, Default, Serialize, Deserialize)]
pub struct Config {
    pub app: Option<String>,
    pub base_url: Option<String>,
}

impl Config {
    pub fn load() -> Result<Self> {
        if !PathBuf::from(CONFIG_PATH).exists() {
            return Ok(Self::default());
        }

        let content = fs::read_to_string(CONFIG_PATH).context("Failed to read config file.")?;
        let config: Config = toml::from_str(&content).context("Failed to parse config file.")?;

        Ok(config)
    }

    pub fn save(&self) -> Result<()> {
        let content = toml::to_string_pretty(self).context("Failed to serialize config")?;
        fs::write(CONFIG_PATH, content).context("Failed to write config file")?;

        Ok(())
    }
}
