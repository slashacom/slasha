use crate::{config::Config, http::client};
use anyhow::{Context, Result};
use models::app::App;
use serde_json::json;

fn build_git_remote_url(slug: &str) -> String {
    let config = Config::load().unwrap();
    format!("{}/git/{}", config.base_url, slug)
}

fn build_ssh_git_url(slug: &str) -> String {
    let config = Config::load().unwrap();

    let host = config
        .base_url
        .trim_start_matches("http://")
        .trim_start_matches("https://")
        .split(':')
        .next()
        .unwrap_or("localhost");

    format!("slasha@{}:{}.git", host, slug)
}

pub async fn handle_create(name: &str) -> Result<()> {
    let response = client()?
        .post("/api/apps", &json!({ "name": name }))
        .await
        .context("Failed to create app")?;

    if !response.status().is_success() {
        let error_body = response
            .text()
            .await
            .unwrap_or_else(|_| "Unknown error".into());
        anyhow::bail!("Failed to create app: {}", error_body);
    }

    let body: serde_json::Value = response.json().await.context("Failed to parse response")?;
    let app: App =
        serde_json::from_value(body["app"].clone()).context("Failed to parse app object")?;

    let git_url = build_git_remote_url(&app.slug);
    let ssh_url = build_ssh_git_url(&app.slug);

    tracing::info!("App created successfully!");
    tracing::info!("  Name:   {}", app.name);
    tracing::info!("  Slug:   {}", app.slug);
    tracing::info!("  Status: {}", app.status);
    tracing::info!("");
    tracing::info!("Git Remote URLs:");
    tracing::info!("  HTTPS: {}", git_url);
    tracing::info!("  SSH:   {}", ssh_url);
    tracing::info!("");
    tracing::info!("To deploy, add a remote and push:");
    tracing::info!("  git remote add slasha {}", ssh_url);
    tracing::info!("  git push -u slasha main");

    Ok(())
}

pub async fn handle_delete(slug: &str) -> Result<()> {
    let response = client()?
        .delete(&format!("/api/apps/{}", slug))
        .await
        .context("Failed to delete app")?;

    if !response.status().is_success() {
        let error_body = response
            .text()
            .await
            .unwrap_or_else(|_| "Unknown error".into());
        anyhow::bail!("Failed to delete app: {}", error_body);
    }

    tracing::info!("App deleted successfully!");

    Ok(())
}

pub async fn handle_info(slug: &str) -> Result<()> {
    let response = client()?
        .get(&format!("/api/apps/{}", slug))
        .await
        .context("Failed to get app info")?;

    if !response.status().is_success() {
        let error_body = response
            .text()
            .await
            .unwrap_or_else(|_| "Unknown error".into());
        anyhow::bail!("Failed to get app info: {}", error_body);
    }

    let body: serde_json::Value = response.json().await.context("Failed to parse response")?;
    let app: App =
        serde_json::from_value(body["app"].clone()).context("Failed to parse app object")?;

    let git_url = build_git_remote_url(&app.slug);
    let ssh_url = build_ssh_git_url(&app.slug);

    tracing::info!("App info:");
    tracing::info!("  Name:   {}", app.name);
    tracing::info!("  Slug:   {}", app.slug);
    tracing::info!("  Status: {}", app.status);
    tracing::info!("");
    tracing::info!("Git Remote URLs:");
    tracing::info!("  HTTPS: {}", git_url);
    tracing::info!("  SSH:   {}", ssh_url);
    tracing::info!("");
    tracing::info!("To deploy, add a remote and push:");
    tracing::info!("  git remote add slasha {}", ssh_url);
    tracing::info!("  git push slasha main");

    Ok(())
}

pub async fn handle_list() -> Result<()> {
    let response = client()?
        .get("/api/apps")
        .await
        .context("Failed to list apps")?;

    if !response.status().is_success() {
        let error_body = response
            .text()
            .await
            .unwrap_or_else(|_| "Unknown error".into());
        anyhow::bail!("Failed to list apps: {}", error_body);
    }

    let body: serde_json::Value = response.json().await.context("Failed to parse response")?;
    let apps: Vec<App> =
        serde_json::from_value(body["apps"].clone()).context("Failed to parse apps array")?;

    if apps.is_empty() {
        tracing::info!("No apps found. Create one with: slasha apps create <name>");
        return Ok(());
    }

    tracing::info!("{:<20} {:<15} {:<10}", "NAME", "SLUG", "STATUS");
    tracing::info!("{}", "-".repeat(45));

    for app in apps {
        tracing::info!("{:<20} {:<15} {:<10}", app.name, app.slug, app.status);
    }

    Ok(())
}
