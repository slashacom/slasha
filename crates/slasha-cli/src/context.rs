use anyhow::{Result, anyhow};

use crate::{
    config::{GlobalConfig, ProjectConfig},
    http::ApiClient,
};

/// Returns the first non-empty string from an iterator of optional strings.
fn first_non_empty(values: impl IntoIterator<Item = Option<String>>) -> Option<String> {
    values.into_iter().flatten().find(|s| !s.trim().is_empty())
}

pub struct Context {
    pub api_client: Option<ApiClient>,
    pub app: Option<String>,
}

impl Context {
    /// Initializes a new [`Context`] with optional CLI flag overrides, falling back to project
    /// configuration and global configuration.
    ///
    /// # Arguments
    ///
    /// * `server_override` - Optional explicit server base URL override string slice.
    /// * `app_override` - Optional explicit application slug override string slice.
    ///
    /// # Returns
    ///
    /// A configured [`Context`] instance.
    pub fn new(server_override: Option<&str>, app_override: Option<&str>) -> Result<Self> {
        let project = ProjectConfig::load().ok();
        let global = GlobalConfig::load().ok();

        let server_url = first_non_empty([
            server_override.map(str::to_owned),
            project.as_ref().and_then(|c| c.server_url.clone()),
            global.and_then(|c| c.server_url),
        ]);

        let app = first_non_empty([app_override.map(str::to_owned), project.and_then(|c| c.app)]);

        Ok(Self {
            api_client: server_url.map(|url| ApiClient::new(&url)).transpose()?,
            app,
        })
    }

    /// Resolves the API client or returns an error prompting the user to configure the server URL.
    ///
    /// # Returns
    ///
    /// A reference to [`ApiClient`].
    pub fn api_client(&self) -> Result<&ApiClient> {
        self.api_client.as_ref().ok_or_else(|| {
            anyhow!(
                "No server URL configured. Run 'slasha auth login' or specify '--server-url <url>'."
            )
        })
    }

    /// Resolves the application slug or returns an error prompting the user to link the directory.
    ///
    /// # Returns
    ///
    /// A string slice referencing the resolved application slug.
    pub fn app(&self) -> Result<&str> {
        self.app.as_deref().ok_or_else(|| {
            anyhow!(
                "No app configured. Run 'slasha link' in this directory or specify '--app <slug>'."
            )
        })
    }

    /// Resolves both the API client and application slug or returns an error if either is unconfigured.
    ///
    /// # Returns
    ///
    /// A tuple containing references to [`ApiClient`] and the application slug string slice.
    pub fn require_context(&self) -> Result<(&ApiClient, &str)> {
        let client = self.api_client()?;
        let app = self.app()?;

        Ok((client, app))
    }
}
