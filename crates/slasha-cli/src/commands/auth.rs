use anyhow::Result;
use colored::Colorize;
use inquire::{Password, PasswordDisplayMode, Text};
use serde::{Deserialize, Serialize};
use serde_json::json;
use slasha_db::user::User;

use crate::{
    clap_app::AuthCommand,
    config::{GlobalConfig, ProjectConfig},
    context::Context,
    http::ApiClient,
    output::{cli_info, cli_label, cli_section, cli_success, spinner},
    token::{clear_auth_token, set_auth_token},
};

#[derive(Deserialize, Serialize)]
struct StatusResponse {
    has_admin: bool,
}

#[derive(Deserialize, Serialize)]
struct AuthResponse {
    token: String,
}

#[derive(Deserialize, Serialize)]
struct MeResponse {
    user: User,
}

pub async fn dispatch(cmd: AuthCommand, server_override: Option<&str>) -> Result<()> {
    match cmd {
        AuthCommand::Login => handle_login(server_override).await,
        AuthCommand::Logout => handle_logout(server_override),
        AuthCommand::Status => {
            let ctx = Context::new(server_override, None)?;
            let client = ctx.api_client()?;
            handle_status(client).await
        }
    }
}

async fn handle_login(server_url_flag: Option<&str>) -> Result<()> {
    let server_url = resolve_server_url(server_url_flag)?;
    let client = ApiClient::new(&server_url)?;
    let status: StatusResponse = client.get("/api/auth/status").await?;

    if !status.has_admin {
        cli_info(format!(
            "{} No admin account exists. Setting up initial admin...",
            "-->".cyan()
        ));

        let email = Text::new("Admin email:").prompt()?;
        let password = Password::new("Password:")
            .with_display_mode(PasswordDisplayMode::Masked)
            .with_display_toggle_enabled()
            .prompt()?;

        let _spin = spinner("Creating admin account...");

        let auth: AuthResponse = client
            .post(
                "/api/auth/signup",
                &json!({
                    "email": email,
                    "password": password,
                    "confirm_password": password,
                }),
            )
            .await?;

        set_auth_token(&server_url, &auth.token)?;
        save_default_server_if_unset(&server_url)?;
        cli_success("Admin account created and logged in.");

        return Ok(());
    }

    let email = Text::new("Email:").prompt()?;
    let password = Password::new("Password:")
        .without_confirmation()
        .with_display_mode(PasswordDisplayMode::Masked)
        .with_display_toggle_enabled()
        .prompt()?;

    let _spin = spinner("Authenticating...");

    let auth: AuthResponse = client
        .post(
            "/api/auth/login",
            &json!({ "email": email, "password": password }),
        )
        .await?;

    set_auth_token(&server_url, &auth.token)?;
    save_default_server_if_unset(&server_url)?;
    cli_success(format!("Logged in successfully on {}.", server_url.cyan()));

    Ok(())
}

fn handle_logout(server_url_flag: Option<&str>) -> Result<()> {
    let url = resolve_server_url(server_url_flag)?;

    clear_auth_token(&url)?;
    cli_success(format!("Logged out from {}.", url.cyan()));

    Ok(())
}

async fn handle_status(client: &ApiClient) -> Result<()> {
    let me: MeResponse = client.get("/api/auth/me").await?;

    cli_section("Current user");
    cli_label("Email", &me.user.email);
    cli_label("Role", me.user.role);
    cli_label("Server", client.base_url());

    Ok(())
}

/// Resolves the target server URL from flag, project config, or global config, prompting as a fallback.
fn resolve_server_url(server_override: Option<&str>) -> Result<String> {
    if let Some(url) = server_override.filter(|s| !s.trim().is_empty()) {
        return Ok(url.to_string());
    }

    if let Some(url) = ProjectConfig::load()
        .ok()
        .and_then(|c| c.server_url)
        .filter(|s| !s.trim().is_empty())
    {
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

/// Saves the logged-in server URL to global config if no default server is currently set.
///
/// # Arguments
///
/// * `server_url` - Target server URL string.
fn save_default_server_if_unset(server_url: &str) -> Result<()> {
    let mut global = GlobalConfig::load().unwrap_or_default();
    if global
        .server_url
        .as_ref()
        .is_none_or(|s| s.trim().is_empty())
    {
        global.server_url = Some(server_url.to_string());
        global.save()?;
    }

    Ok(())
}
