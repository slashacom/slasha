use std::{env, process::Command};

use anyhow::{Context, Result};
use slasha_db::{
    create_pool_with_max_size,
    repos::{app::AppRepo, user::UserRepo},
};

pub async fn handle(user_id: String) -> Result<i32> {
    let db_path = dirs::home_dir()
        .context("Failed to get home directory")?
        .join(".slasha")
        .join("slasha.db");

    let pool = create_pool_with_max_size(db_path.to_str().context("Invalid DB path")?, 1)
        .context("Failed to create database pool")?;

    let user = UserRepo::find_by_id(&pool, &user_id)
        .await
        .context("Failed to verify user")?;

    let ssh_command = env::var("SSH_ORIGINAL_COMMAND").context("SSH_ORIGINAL_COMMAND not set")?;
    let mut parts = ssh_command.split_whitespace();

    let service = parts
        .next()
        .context("Git service not found in SSH_ORIGINAL_COMMAND")?;

    let repo_slug = parts
        .next()
        .context("Repository path not found in SSH_ORIGINAL_COMMAND")?
        .trim_matches(|c| c == '\'' || c == '"' || c == '/');

    let repo_slug = repo_slug.strip_suffix(".git").unwrap_or(repo_slug);

    let app = AppRepo::find_by_slug_for_user(&pool, repo_slug, &user.id)
        .await
        .context("Access denied or repository not found")?;

    if !["git-upload-pack", "git-receive-pack"].contains(&service) {
        anyhow::bail!("Unsupported Git service: {}", service);
    }
    if !app.source.accepts_pushes() && service == "git-receive-pack" {
        anyhow::bail!("Externally sourced apps do not accept direct pushes");
    }

    let service = service.trim_start_matches("git-");

    let mut child = Command::new("git")
        .arg(service)
        .arg(&app.repo_path)
        .spawn()
        .context("Failed to spawn git process")?;

    let status = child.wait().context("Failed to wait on git process")?;
    Ok(status.code().unwrap_or(1))
}
