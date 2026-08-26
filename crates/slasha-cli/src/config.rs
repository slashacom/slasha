use std::{fs, path::PathBuf};

use anyhow::{Context as _, Result};
use serde::{Deserialize, Serialize};

use crate::{clap_app::ConfigCommand, output::cli_success};

pub const DEFAULT_BASE_URL: &str = "http://localhost:3000";

const PROJECT_CONFIG_PATH: &str = "slasha.toml";
const GLOBAL_CONFIG_FILE: &str = "config.toml";
const GLOBAL_CONFIG_DIR: &str = "slasha";

/// Project-level configuration stored in `slasha.toml` in the current working directory.
#[derive(Debug, Default, Serialize, Deserialize)]
pub struct ProjectConfig {
    /// Linked application slug.
    pub app: Option<String>,
}

impl ProjectConfig {
    /// Loads `slasha.toml` configuration from the current working directory.
    ///
    /// # Returns
    ///
    /// The loaded [`ProjectConfig`] struct.
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

    /// Saves current project configuration to `slasha.toml` in the current working directory.
    pub fn save(&self) -> Result<()> {
        let content = toml::to_string_pretty(self).context("Failed to serialize slasha.toml")?;
        fs::write(PROJECT_CONFIG_PATH, content).context("Failed to write slasha.toml")?;

        Ok(())
    }
}

/// User-level global configuration stored in `config.toml` under the user configuration directory.
#[derive(Debug, Default, Serialize, Deserialize)]
pub struct GlobalConfig {
    /// Persisted server base URL.
    pub base_url: Option<String>,
}

impl GlobalConfig {
    /// Resolves the filesystem path to the user's global `config.toml` file.
    ///
    /// # Returns
    ///
    /// A [`PathBuf`] pointing to the global config file path.
    pub fn path() -> Result<PathBuf> {
        let dir = dirs::config_dir()
            .context("Failed to resolve user config directory")?
            .join(GLOBAL_CONFIG_DIR);

        Ok(dir.join(GLOBAL_CONFIG_FILE))
    }

    /// Loads user global configuration from `config.toml`.
    ///
    /// # Returns
    ///
    /// The loaded [`GlobalConfig`] struct.
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

    /// Saves global configuration settings to `config.toml`.
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

pub async fn dispatch(cmd: ConfigCommand) -> Result<()> {
    match cmd {
        ConfigCommand::SetUrl { url } => handle_set_url(&url).await,
    }
}

pub async fn handle_set_url(url: &str) -> Result<()> {
    let mut config = GlobalConfig::load()?;
    config.base_url = Some(url.to_string());
    config.save()?;

    cli_success(format!("Base URL saved: {}", url));
    Ok(())
}
