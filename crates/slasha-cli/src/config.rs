use std::{fs, path::PathBuf};

use anyhow::{Context, Result};
use serde::{Deserialize, Serialize};

pub const DEFAULT_BASE_URL: &str = "http://localhost:3000";

const PROJECT_CONFIG_PATH: &str = "slasha.toml";
const GLOBAL_CONFIG_FILE: &str = "config.toml";
const GLOBAL_CONFIG_DIR: &str = "slasha";

#[derive(Debug, Default, Serialize, Deserialize)]
pub struct ProjectConfig {
    pub app: Option<String>,
}

impl ProjectConfig {
    pub fn load() -> Result<Self> {
        if !PathBuf::from(PROJECT_CONFIG_PATH).exists() {
            return Ok(Self::default());
        }

        let content =
            fs::read_to_string(PROJECT_CONFIG_PATH).context("Failed to read slasha.toml")?;
        let config: ProjectConfig =
            toml::from_str(&content).context("Failed to parse slasha.toml")?;

        Ok(config)
    }

    pub fn save(&self) -> Result<()> {
        let content = toml::to_string_pretty(self).context("Failed to serialize slasha.toml")?;
        fs::write(PROJECT_CONFIG_PATH, content).context("Failed to write slasha.toml")?;

        Ok(())
    }
}

#[derive(Debug, Default, Serialize, Deserialize)]
pub struct GlobalConfig {
    pub base_url: Option<String>,
    pub git_host: Option<String>,
}

impl GlobalConfig {
    pub fn path() -> Result<PathBuf> {
        let dir = dirs::config_dir()
            .context("Failed to resolve user config directory")?
            .join(GLOBAL_CONFIG_DIR);

        Ok(dir.join(GLOBAL_CONFIG_FILE))
    }

    pub fn load() -> Result<Self> {
        let path = Self::path()?;
        if !path.exists() {
            return Ok(Self::default());
        }

        let content = fs::read_to_string(&path)
            .with_context(|| format!("Failed to read {}", path.display()))?;
        let config: GlobalConfig = toml::from_str(&content)
            .with_context(|| format!("Failed to parse {}", path.display()))?;

        Ok(config)
    }

    pub fn save(&self) -> Result<()> {
        let path = Self::path()?;
        if let Some(parent) = path.parent() {
            fs::create_dir_all(parent).with_context(|| {
                format!("Failed to create config directory {}", parent.display())
            })?;
        }

        let content = toml::to_string_pretty(self).context("Failed to serialize global config")?;
        fs::write(&path, content).with_context(|| format!("Failed to write {}", path.display()))?;

        Ok(())
    }
}
