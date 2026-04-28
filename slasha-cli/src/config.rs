use std::{fs, path::PathBuf};

use anyhow::{Context, Result};
use serde::{Deserialize, Serialize};

#[derive(Debug, Default, Serialize, Deserialize)]
pub struct Config {
    pub base_url: String,
    pub auth_token: Option<String>,
}

impl Config {
    pub fn path() -> Result<PathBuf> {
        dirs::home_dir()
            .context("Failed to get home directory")
            .map(|p| p.join(".slasha/config.json"))
    }

    pub fn load() -> Result<Self> {
        let path = Self::path()?;

        if !path.exists() {
            return Ok(Self::default());
        }

        let content = fs::read_to_string(&path)
            .with_context(|| format!("failed to read config file: {}", path.display()))?;

        let mut config: Config = serde_json::from_str(&content)
            .with_context(|| format!("failed to parse config file: {}", path.display()))?;

        if config.base_url.is_empty() {
            config.base_url = "http://localhost:3000".to_string();
        }

        Ok(config)
    }

    pub fn save(&self) -> Result<()> {
        let path = Self::path()?;

        if let Some(parent) = path.parent() {
            fs::create_dir_all(parent)
                .with_context(|| format!("failed to create config dir: {}", parent.display()))?;
        }

        let content = serde_json::to_string_pretty(self).context("failed to serialize config")?;

        fs::write(&path, content)
            .with_context(|| format!("failed to write config file: {}", path.display()))?;

        Ok(())
    }
}
