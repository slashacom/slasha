use anyhow::Result;
use colored::Colorize;

use crate::{
    commands::apps::{AppDetailsResponse, AppListResponse},
    config::{GlobalConfig, ProjectConfig},
    http::ApiClient,
    output::{cli_success, spinner},
    token::get_auth_token,
};

pub async fn handle_link(app_override: Option<&str>, server_override: Option<&str>) -> Result<()> {
    let mut config = ProjectConfig::load().unwrap_or_default();

    let server_url = resolve_server_url(server_override, &config)?;

    require_authenticated(&server_url)?;

    let client = ApiClient::new(&server_url)?;

    let slug = match (app_override, config.app.clone()) {
        (Some(s), _) => {
            let _spin = spinner("Verifying application...");
            let _: AppDetailsResponse = client.get(&format!("/api/apps/{}", s)).await?;
            s.to_string()
        }
        (None, None) => select_app(&client, &server_url).await?,
        (None, Some(existing_app)) => {
            let prompt_msg = format!(
                "Currently linked to app '{}'. Change linked application?",
                existing_app.cyan()
            );

            if inquire::Confirm::new(&prompt_msg)
                .with_default(false)
                .prompt()?
            {
                select_app(&client, &server_url).await?
            } else {
                existing_app
            }
        }
    };

    let global_default = GlobalConfig::load().ok().and_then(|g| g.server_url);
    let server_differs_from_global = global_default
        .as_deref()
        .map(|d| d.trim_end_matches('/') != server_url.trim_end_matches('/'))
        .unwrap_or(true);

    config.app = Some(slug.clone());
    if server_differs_from_global {
        config.server_url = Some(server_url.clone());
    } else {
        config.server_url = None;
    }
    config.save()?;

    cli_success(format!(
        "Linked current directory to app '{}' at {} in .slasha/config.toml",
        slug.cyan(),
        server_url.cyan()
    ));

    Ok(())
}

/// Resolves the server URL from flag, project config, or global config, prompting as a fallback.
///
/// # Arguments
///
/// * `flag` - Optional explicit server URL flag.
/// * `config` - Current project config reference ([`ProjectConfig`]).
///
/// # Returns
///
/// The resolved server URL string.
fn resolve_server_url(server_override: Option<&str>, config: &ProjectConfig) -> Result<String> {
    if let Some(url) = server_override.filter(|s| !s.trim().is_empty()) {
        return Ok(url.to_string());
    }

    if let Some(url) = config.server_url.clone().filter(|s| !s.trim().is_empty()) {
        return Ok(url);
    }

    if let Some(url) = GlobalConfig::load()
        .ok()
        .and_then(|g| g.server_url)
        .filter(|s| !s.trim().is_empty())
    {
        return Ok(url);
    }

    let url = inquire::Text::new("Server URL:")
        .with_default(crate::config::DEFAULT_BASE_URL)
        .prompt()?;

    Ok(url)
}

/// Checks that a valid auth token exists for server_url.
fn require_authenticated(server_url: &str) -> Result<()> {
    if get_auth_token(server_url)?.is_none() {
        anyhow::bail!(
            "not authenticated for {}\nhint: run `slasha auth login --server-url {}` first",
            server_url,
            server_url
        );
    }
    Ok(())
}

/// Fetches the app list from the server and prompts the user to select one.
async fn select_app(client: &ApiClient, server_url: &str) -> Result<String> {
    let res: AppListResponse = {
        let _spin = spinner("Fetching available applications...");
        client.get("/api/apps").await?
    };

    if res.apps.is_empty() {
        anyhow::bail!(
            "No applications found on {}. Create one first with `slasha apps create <name>`.",
            server_url
        );
    }

    let choices: Vec<String> = res.apps.into_iter().map(|a| a.app.slug).collect();
    let slug = inquire::Select::new("Select application to link:", choices).prompt()?;
    Ok(slug)
}
