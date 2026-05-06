use anyhow::{Context, Result};
use colored::Colorize;
use serde_json::json;
use slasha_db::user::User;

use crate::{
    clap_app::UsersCommand,
    output::{cli_label, cli_success, confirm_action, output, print_table},
    state::AppState,
};

pub async fn dispatch(state: &AppState, cmd: UsersCommand) -> Result<()> {
    match cmd {
        UsersCommand::List => handle_list(state).await,
        UsersCommand::Create {
            email,
            password,
            role,
        } => handle_create(state, &email, &password, &role).await,
        UsersCommand::Update { id, email, role } => handle_update(state, &id, email, role).await,
        UsersCommand::Delete { id, yes } => handle_delete(state, &id, yes).await,
    }
}

pub async fn handle_list(state: &AppState) -> Result<()> {
    let users_data = state.client.get("/api/users").await?;

    let users: Vec<User> =
        serde_json::from_value(users_data["users"].clone()).context("Failed to parse users")?;

    output(state.output, &users, || {
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
        .client
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

    output(state.output, &user, || {
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
        .client
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

    output(state.output, &user, || {
        cli_success("User updated.");
        cli_label("Email", &user.email);
        cli_label("Role", &user.role);
    })?;

    Ok(())
}

pub async fn handle_delete(state: &AppState, id: &str, yes: bool) -> Result<()> {
    if !confirm_action(state.output, yes, &format!("Delete user {}?", id.red()))? {
        return Ok(());
    }

    state.client.delete(&format!("/api/users/{}", id)).await?;

    output(state.output, &json!({ "ok": true }), || {
        cli_success("User deleted.");
    })?;

    Ok(())
}
