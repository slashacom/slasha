use std::{fs, path::PathBuf};

use anyhow::{Context as _, Result};
use serde::{Deserialize, Serialize};

pub const DEFAULT_BASE_URL: &str = "http://localhost:3000";

const CONFIG_DIR: &str = ".slasha";
const CONFIG_FILE: &str = "config.toml";

/// Global user configuration stored in the OS config directory.
#[derive(Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct GlobalConfig {
    /// Default server URL to use when not specified in project config.
    #[serde(rename = "default-server", skip_serializing_if = "Option::is_none")]
    pub default_server: Option<String>,
}

impl GlobalConfig {
    /// Returns the path to the global config file.
    fn path() -> Result<PathBuf> {
        let dir = dirs::config_dir()
            .context("Could not find configuration directory")?
            .join("slasha");
        Ok(dir.join(CONFIG_FILE))
    }

    /// Loads the global configuration from the OS config directory.
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

    /// Saves the global configuration to the OS config directory.
    pub fn save(&self) -> Result<()> {
        let path = Self::path()?;
        if let Some(parent) = path.parent() {
            fs::create_dir_all(parent)
                .with_context(|| format!("Failed to create directory {}", parent.display()))?;
        }

        let content = toml::to_string_pretty(self).context("Failed to serialize global config")?;
        fs::write(&path, content).with_context(|| format!("Failed to write {}", path.display()))?;

        Ok(())
    }
}

/// Project-level configuration stored in `.slasha/config.toml` in the current working directory.
#[derive(Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct ProjectConfig {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub app: Option<String>,
    #[serde(rename = "server-url", skip_serializing_if = "Option::is_none")]
    pub server_url: Option<String>,
}

impl ProjectConfig {
    /// Returns the path to `.slasha/config.toml` relative to the current directory.
    ///
    /// # Returns
    ///
    /// A [`PathBuf`] representing `.slasha/config.toml`.
    fn path() -> PathBuf {
        PathBuf::from(CONFIG_DIR).join(CONFIG_FILE)
    }

    /// Loads `.slasha/config.toml` configuration from the current working directory.
    ///
    /// # Returns
    ///
    /// The loaded [`ProjectConfig`] struct, or default if missing.
    pub fn load() -> Result<Self> {
        let path = Self::path();
        if !path.exists() {
            return Ok(Self::default());
        }

        let content = fs::read_to_string(&path)
            .with_context(|| format!("Failed to read {}", path.display()))?;
        let config: ProjectConfig = toml::from_str(&content)
            .with_context(|| format!("Failed to parse {}", path.display()))?;

        Ok(config)
    }

    /// Saves current project configuration to `.slasha/config.toml` in the current working directory.
    ///
    /// # Returns
    ///
    /// Ok(()) on success.
    pub fn save(&self) -> Result<()> {
        let path = Self::path();
        if let Some(parent) = path.parent() {
            fs::create_dir_all(parent)
                .with_context(|| format!("Failed to create directory {}", parent.display()))?;
        }

        let content = toml::to_string_pretty(self).context("Failed to serialize project config")?;
        fs::write(&path, content).with_context(|| format!("Failed to write {}", path.display()))?;

        Ok(())
    }
}
