use anyhow::{Context, Result};
use colored::Colorize;
use serde_json::json;
use slasha_db::app::{App, AppSource};

use crate::{
    config::ProjectConfig,
    output::{cli_info, cli_label, cli_section, cli_success, confirm_action, output, print_table},
    state::AppState,
};

fn git_remote_url(state: &AppState, slug: &str) -> String {
    format!("{}/git/{}", state.api_client.base_url(), slug)
}

fn ssh_git_url(state: &AppState, slug: &str) -> String {
    format!("slasha@{}:{}.git", state.api_client.git_host(), slug)
}

fn print_app(state: &AppState, app: &App, status: &str) {
    cli_section(&app.name);
    cli_label("Slug", &app.slug);
    cli_label("Status", status);
    cli_label("Branch", &app.default_branch);
    cli_label("Source", app.source.to_string());

    match app.source {
        AppSource::Local => {
            cli_section("Git remotes");
            cli_label("HTTPS", git_remote_url(state, &app.slug));
            cli_label("SSH", ssh_git_url(state, &app.slug));

            cli_section("Deploy");
            cli_info(format!(
                "  git remote add slasha {}",
                ssh_git_url(state, &app.slug)
            ));
            cli_info("  git push -u slasha main".to_string());
        }
        AppSource::Github => {
            cli_section("Deploy");
            cli_info("Push to the connected GitHub repository or deploy from the dashboard.");
        }
        AppSource::Git => {
            cli_section("Deploy");
            cli_info("Deployments are triggered from the dashboard.");
        }
    }
}

pub async fn handle_list(state: &AppState) -> Result<()> {
    let body = state.api_client.get("/api/apps").await?;

    let items = body["apps"]
        .as_array()
        .context("Expected apps to be an array")?;

    output(state.output_mode, &body["apps"], || {
        if items.is_empty() {
            cli_info("No apps yet. Run slasha create <name> to create one.");
        } else {
            let mut rows = Vec::new();
            for item in items {
                let app: App = serde_json::from_value(item["app"].clone()).unwrap();
                let status = item["runtime_status"].as_str().unwrap_or("idle");
                rows.push(vec![
                    app.name,
                    app.slug,
                    status.to_string(),
                    app.default_branch,
                    app.source.to_string(),
                ]);
            }
            print_table(&["NAME", "SLUG", "STATUS", "BRANCH", "SOURCE"], rows);
        }
    })?;

    Ok(())
}

pub async fn handle_create(state: &AppState, name: &str) -> Result<()> {
    let body = state
        .api_client
        .post("/api/apps", &json!({ "name": name, "source": "local" }))
        .await?;

    let app: App = serde_json::from_value(body["app"].clone()).context("Failed to parse app")?;
    let status = body["runtime_status"].as_str().unwrap_or("idle");

    output(state.output_mode, &app, || {
        cli_success("App created");
        print_app(state, &app, status);
    })?;

    Ok(())
}

pub async fn handle_info(state: &AppState, slug: &str) -> Result<()> {
    let body = state.api_client.get(&format!("/api/apps/{}", slug)).await?;

    let app: App = serde_json::from_value(body["app"].clone()).context("Failed to parse app")?;
    let status = body["runtime_status"].as_str().unwrap_or("idle");

    output(state.output_mode, &app, || {
        print_app(state, &app, status);
    })?;

    Ok(())
}

pub async fn handle_delete(state: &AppState, slug: &str, yes: bool) -> Result<()> {
    if !confirm_action(
        state.output_mode,
        yes,
        &format!(
            "Delete app {}? This removes all deployments and services.",
            slug.red()
        ),
    )? {
        return Ok(());
    }

    state
        .api_client
        .delete(&format!("/api/apps/{}", slug))
        .await?;

    output(
        state.output_mode,
        &json!({ "ok": true, "slug": slug }),
        || {
            cli_success(format!("App {} deleted.", slug));
        },
    )?;

    Ok(())
}

pub async fn handle_link(state: &AppState, app_flag: Option<String>) -> Result<()> {
    let slug = match app_flag {
        Some(s) => s,
        None => {
            let body = state.api_client.get("/api/apps").await?;

            let apps: Vec<App> =
                serde_json::from_value(body["apps"].clone()).context("Failed to parse apps")?;

            if apps.is_empty() {
                anyhow::bail!("No apps found. Create one first with `slasha create <name>`.");
            }

            let choices: Vec<String> = apps.into_iter().map(|a| a.slug).collect();
            inquire::Select::new("Select app to link:", choices).prompt()?
        }
    };

    state.api_client.get(&format!("/api/apps/{}", slug)).await?;

    let mut config = ProjectConfig::load().unwrap_or_default();
    config.app = Some(slug.clone());
    config.save()?;

    output(
        state.output_mode,
        &json!({ "ok": true, "slug": slug }),
        || {
            cli_success(format!(
                "Linked current directory to app '{}' in slasha.toml",
                slug
            ));
        },
    )?;

    Ok(())
}
