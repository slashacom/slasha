use anyhow::{Context, Result};
use colored::Colorize;
use inquire::{Password, PasswordDisplayMode, Text};
use serde::{Deserialize, Serialize};
use serde_json::json;
use slasha_db::user::User;

use crate::{
    output::{cli_info, cli_label, cli_section, cli_success, output, spinner},
    state::AppState,
    token::{clear_auth_token, set_auth_token},
};

#[derive(Deserialize, Serialize)]
struct AuthResponse {
    token: String,
}

#[derive(Deserialize, Serialize)]
struct StatusResponse {
    has_admin: bool,
}

#[derive(Deserialize, Serialize)]
struct MeResponse {
    user: User,
}

async fn check_has_admin(state: &AppState) -> Result<bool> {
    let status: StatusResponse =
        serde_json::from_value(state.api_client.get("/api/auth/status").await?)
            .context("Failed to parse status")?;

    Ok(status.has_admin)
}

pub async fn handle_login(state: &AppState) -> Result<()> {
    let has_admin = check_has_admin(state).await?;

    if !has_admin {
        if !state.output_mode.is_json() {
            cli_info(format!(
                "{} No admin account exists. Setting up initial admin...",
                "-->".cyan()
            ));
        }
        return handle_signup(state).await;
    }

    let email = Text::new("Email:").prompt()?;
    let password = Password::new("Password:")
        .without_confirmation()
        .with_display_mode(PasswordDisplayMode::Masked)
        .with_display_toggle_enabled()
        .prompt()?;

    let pb = spinner("Authenticating...");

    let auth: AuthResponse = match serde_json::from_value(
        state
            .api_client
            .post(
                "/api/auth/login",
                &json!({ "email": email, "password": password }),
            )
            .await?,
    ) {
        Ok(v) => v,
        Err(e) => {
            pb.finish_and_clear();
            anyhow::bail!("Login failed: {}", e);
        }
    };

    set_auth_token(&auth.token)?;

    pb.finish_and_clear();

    output(
        state.output_mode,
        &json!({ "ok": true, "message": "Logged in" }),
        || {
            cli_success("Logged in successfully.");
        },
    )?;

    Ok(())
}

async fn handle_signup(state: &AppState) -> Result<()> {
    let email = Text::new("Admin email:").prompt()?;
    let password = Password::new("Password:")
        .with_display_mode(PasswordDisplayMode::Masked)
        .with_display_toggle_enabled()
        .prompt()?;

    let pb = spinner("Creating admin account...");

    let auth: AuthResponse = match serde_json::from_value(
        state
            .api_client
            .post(
                "/api/auth/signup",
                &json!({
                    "email": email,
                    "password": password,
                    "confirm_password": password, // inquire already handles confirmation
                }),
            )
            .await?,
    ) {
        Ok(v) => v,
        Err(e) => {
            pb.finish_and_clear();
            anyhow::bail!("Signup failed: {}", e);
        }
    };

    set_auth_token(&auth.token)?;

    pb.finish_and_clear();

    output(
        state.output_mode,
        &json!({ "ok": true, "message": "Admin account created" }),
        || {
            cli_success("Admin account created and logged in.");
        },
    )?;

    Ok(())
}

pub async fn handle_logout(state: &AppState) -> Result<()> {
    clear_auth_token()?;

    output(state.output_mode, &json!({ "ok": true }), || {
        cli_success("Logged out.");
    })?;

    Ok(())
}

pub async fn handle_me(state: &AppState) -> Result<()> {
    let me: MeResponse = serde_json::from_value(state.api_client.get("/api/auth/me").await?)
        .context("Failed to parse me response")?;

    output(state.output_mode, &me.user, || {
        cli_section("Current user");
        cli_label("Email", &me.user.email);
        cli_label("Role", me.user.role);
    })?;

    Ok(())
}

pub async fn handle_status(state: &AppState) -> Result<()> {
    let health = state.api_client.get("/api/health").await?;

    output(state.output_mode, &health, || {
        cli_success(format!(
            "Server is {}",
            health["status"].as_str().unwrap_or("ok").green()
        ));
        cli_label("Version", &health["version"]);
    })?;

    Ok(())
}
