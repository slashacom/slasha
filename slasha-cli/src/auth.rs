use crate::{config::Config, http::client};
use anyhow::{Context, Result};
use indicatif::{ProgressBar, ProgressStyle};
use inquire::{Password, PasswordDisplayMode, Text};
use models::user::User;
use serde::Deserialize;
use serde_json::json;
use std::time::Duration;

#[derive(Deserialize)]
struct AuthResponse {
    token: String,
}

pub async fn handle_login() -> Result<()> {
    tracing::info!("Login to Slasha");
    let email = Text::new("Email:").prompt()?;
    let password = Password::new("Password:")
        .without_confirmation()
        .with_display_mode(PasswordDisplayMode::Masked)
        .with_display_toggle_enabled()
        .prompt()?;

    let pb = ProgressBar::new_spinner();
    pb.enable_steady_tick(Duration::from_millis(100));
    pb.set_style(
        ProgressStyle::default_spinner()
            .tick_strings(&["-", "\\", "|", "/"])
            .template("{spinner} {msg}")?,
    );
    pb.set_message("Authenticating...");

    let response = client()?
        .post(
            "/api/auth/login",
            &json!({ "email": email, "password": password }),
        )
        .await
        .context("Login request failed")?;

    if !response.status().is_success() {
        pb.finish_and_clear();
        let error_body = response
            .text()
            .await
            .unwrap_or_else(|_| "Unknown error".into());
        anyhow::bail!("Login failed: {}", error_body);
    }

    let auth_res: AuthResponse = response.json().await.context("Failed to parse response")?;

    let mut conf = Config::load()?;
    conf.auth_token = Some(auth_res.token);
    conf.save()?;

    pb.finish_with_message("Successfully logged in.");
    Ok(())
}

pub async fn handle_me() -> Result<()> {
    let response = client()?
        .get("/api/auth/me")
        .await
        .context("Failed to get user info")?;

    if !response.status().is_success() {
        let error_body = response
            .text()
            .await
            .unwrap_or_else(|_| "Unknown error".into());
        anyhow::bail!("Failed to get user info: {}", error_body);
    }

    let value: serde_json::Value = response.json().await.context("Failed to parse response")?;
    let user: User = serde_json::from_value(value["user"].clone())
        .context("Failed to deserialize user object")?;

    tracing::info!("Email: {}", user.email);
    tracing::info!("Role: {}", user.role);

    Ok(())
}
