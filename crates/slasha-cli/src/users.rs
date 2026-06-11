use std::io::{IsTerminal, Read};

use anyhow::{Context, Result};
use colored::Colorize;
use inquire::{Password, PasswordDisplayMode};
use serde_json::json;
use slasha_db::user::User;

use crate::{
    clap_app::UsersCommand,
    output::{cli_label, cli_success, confirm_action, output, print_table},
    state::AppState,
};

const PASSWORD_ENV: &str = "SLASHA_PASSWORD";

pub async fn dispatch(state: &AppState, cmd: UsersCommand) -> Result<()> {
    match cmd {
        UsersCommand::List => handle_list(state).await,
        UsersCommand::Create {
            email,
            password_stdin,
            role,
        } => {
            let password = read_password(password_stdin)?;
            handle_create(state, &email, &password, &role).await
        }
        UsersCommand::Update { id, email, role } => handle_update(state, &id, email, role).await,
        UsersCommand::Delete { id, yes } => handle_delete(state, &id, yes).await,
    }
}

fn read_password(from_stdin: bool) -> Result<String> {
    if from_stdin {
        let mut buf = String::new();
        std::io::stdin()
            .read_to_string(&mut buf)
            .context("Failed to read password from stdin")?;

        let password = buf.trim_end_matches(['\n', '\r']).to_string();
        if password.is_empty() {
            anyhow::bail!("Empty password received on stdin");
        }
        return Ok(password);
    }

    if let Ok(env_password) = std::env::var(PASSWORD_ENV)
        && !env_password.is_empty()
    {
        return Ok(env_password);
    }

    if !std::io::stdin().is_terminal() {
        anyhow::bail!(
            "No TTY available — pass --password-stdin or set {} to provide the password",
            PASSWORD_ENV
        );
    }

    Password::new("Password:")
        .with_display_mode(PasswordDisplayMode::Masked)
        .with_display_toggle_enabled()
        .prompt()
        .context("Failed to read password")
}

pub async fn handle_list(state: &AppState) -> Result<()> {
    let users_data = state.api_client.get("/api/users").await?;

    let users: Vec<User> =
        serde_json::from_value(users_data["users"].clone()).context("Failed to parse users")?;

    output(state.output_mode, &users, || {
        print_table(
            &["ID", "EMAIL", "ROLE", "CREATED AT"],
            users
                .iter()
                .map(|u| {
                    vec![
                        u.id.to_string(),
                        u.email.clone(),
                        u.role.clone(),
                        u.created_at.format("%Y-%m-%d").to_string(),
                    ]
                })
                .collect(),
        );
    })?;

    Ok(())
}

pub async fn handle_create(
    state: &AppState,
    email: &str,
    password: &str,
    role: &str,
) -> Result<()> {
    let create_res = state
        .api_client
        .post(
            "/api/users",
            &json!({
                "email": email,
                "password": password,
                "role": role,
            }),
        )
        .await?;

    let user: User =
        serde_json::from_value(create_res["user"].clone()).context("Failed to parse user")?;

    output(state.output_mode, &user, || {
        cli_success("User created.");
        cli_label("ID", &user.id);
        cli_label("Email", &user.email);
        cli_label("Role", &user.role);
    })?;

    Ok(())
}

pub async fn handle_update(
    state: &AppState,
    id: &str,
    email: Option<String>,
    role: Option<String>,
) -> Result<()> {
    let update_res = state
        .api_client
        .put(
            &format!("/api/users/{}", id),
            &json!({
                "email": email,
                "role": role,
            }),
        )
        .await?;

    let user: User =
        serde_json::from_value(update_res["user"].clone()).context("Failed to parse user")?;

    output(state.output_mode, &user, || {
        cli_success("User updated.");
        cli_label("Email", &user.email);
        cli_label("Role", &user.role);
    })?;

    Ok(())
}

pub async fn handle_delete(state: &AppState, id: &str, yes: bool) -> Result<()> {
    if !confirm_action(
        state.output_mode,
        yes,
        &format!("Delete user {}?", id.red()),
    )? {
        return Ok(());
    }

    state
        .api_client
        .delete(&format!("/api/users/{}", id))
        .await?;

    output(state.output_mode, &json!({ "ok": true }), || {
        cli_success("User deleted.");
    })?;

    Ok(())
}
