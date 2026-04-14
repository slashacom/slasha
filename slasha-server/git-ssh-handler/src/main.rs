use std::env;
use std::process::{Command, Stdio};

use anyhow::{Context, Result};
use diesel::prelude::*;
use diesel::sqlite::SqliteConnection;
use models::app::{App, AppMember};
use models::user::User;

fn main() -> Result<()> {
    let user_id = env::args().nth(1).context("No user ID provided")?;

    let db_path = dirs::home_dir()
        .context("Failed to get home directory")?
        .join(".slasha")
        .join("slasha.db");

    let mut conn = SqliteConnection::establish(db_path.to_str().unwrap())
        .context("Failed to connect to database")?;

    let user = models::schema::users::table
        .filter(models::schema::users::id.eq(&user_id))
        .first::<User>(&mut conn)
        .optional()
        .context("Failed to query user")?
        .ok_or_else(|| anyhow::anyhow!("User not found"))?;

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

    let app = models::schema::apps::table
        .filter(models::schema::apps::slug.eq(repo_slug))
        .first::<App>(&mut conn)
        .optional()
        .context("Failed to query app")?
        .ok_or_else(|| anyhow::anyhow!("App not found: {}", repo_slug))?;

    let is_member = models::schema::app_members::table
        .filter(models::schema::app_members::app_id.eq(&app.id))
        .filter(models::schema::app_members::user_id.eq(&user.id))
        .first::<AppMember>(&mut conn)
        .optional()
        .context("Failed to query app membership")?
        .is_some();

    if !is_member {
        anyhow::bail!("User {} cannot access repository {}", user.id, repo_slug);
    }

    if !["git-upload-pack", "git-receive-pack"].contains(&service) {
        anyhow::bail!("Unsupported Git service: {}", service);
    }

    let service = service.trim_start_matches("git-");

    let mut child = Command::new("git")
        .arg(service)
        .arg(&app.repo_path)
        .stdin(Stdio::inherit())
        .stdout(Stdio::inherit())
        .stderr(Stdio::inherit())
        .spawn()
        .context("Failed to spawn git process")?;

    let status = child.wait().context("Failed to wait on git process")?;
    std::process::exit(status.code().unwrap_or(1));
}
