use anyhow::Result;
use colored::Colorize;
use inquire::{Password, PasswordDisplayMode, Text};
use serde::{Deserialize, Serialize};
use serde_json::json;
use slasha_db::user::User;

use crate::{
    context::Context,
    output::{cli_info, cli_label, cli_section, cli_success, spinner},
    token::{clear_auth_token, set_auth_token},
};

#[derive(Deserialize, Serialize)]
struct StatusResponse {
    has_admin: bool,
}

/// Checks whether an admin account has been provisioned on the remote server.
///
/// # Arguments
///
/// * `ctx` - Execution context ([`Context`]).
///
/// # Returns
///
/// `true` if an admin exists, `false` otherwise.
async fn check_has_admin(ctx: &Context) -> Result<bool> {
    let status: StatusResponse = ctx.api_client.get("/api/auth/status").await?;
    Ok(status.has_admin)
}

#[derive(Deserialize, Serialize)]
struct AuthResponse {
    token: String,
}

pub async fn handle_login(ctx: &Context) -> Result<()> {
    let has_admin = check_has_admin(ctx).await?;

    if !has_admin {
        cli_info(format!(
            "{} No admin account exists. Setting up initial admin...",
            "-->".cyan()
        ));
        return handle_signup(ctx).await;
    }

    let email = Text::new("Email:").prompt()?;
    let password = Password::new("Password:")
        .without_confirmation()
        .with_display_mode(PasswordDisplayMode::Masked)
        .with_display_toggle_enabled()
        .prompt()?;

    let _spin = spinner("Authenticating...");

    let auth: AuthResponse = ctx
        .api_client
        .post(
            "/api/auth/login",
            &json!({ "email": email, "password": password }),
        )
        .await?;

    set_auth_token(&auth.token)?;
    cli_success("Logged in successfully.");

    Ok(())
}

async fn handle_signup(ctx: &Context) -> Result<()> {
    if !std::io::IsTerminal::is_terminal(&std::io::stdout()) {
        anyhow::bail!(
            "Cannot prompt for admin creation credentials in non-interactive environment."
        );
    }

    let email = Text::new("Admin email:").prompt()?;
    let password = Password::new("Password:")
        .with_display_mode(PasswordDisplayMode::Masked)
        .with_display_toggle_enabled()
        .prompt()?;

    let _spin = spinner("Creating admin account...");

    let auth: AuthResponse = ctx
        .api_client
        .post(
            "/api/auth/signup",
            &json!({
                "email": email,
                "password": password,
                "confirm_password": password,
            }),
        )
        .await?;

    set_auth_token(&auth.token)?;
    cli_success("Admin account created and logged in.");

    Ok(())
}

pub async fn handle_logout() -> Result<()> {
    clear_auth_token()?;
    cli_success("Logged out.");
    Ok(())
}

#[derive(Deserialize, Serialize)]
struct MeResponse {
    user: User,
}

pub async fn handle_me(ctx: &Context) -> Result<()> {
    let me: MeResponse = ctx.api_client.get("/api/auth/me").await?;

    cli_section("Current user");
    cli_label("Email", &me.user.email);
    cli_label("Role", me.user.role);

    Ok(())
}
