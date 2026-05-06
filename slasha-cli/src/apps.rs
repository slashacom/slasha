use anyhow::{Context, Result};
use colored::Colorize;
use serde_json::json;
use slasha_db::app::App;

use crate::{
    config::Config,
    output::{cli_info, cli_label, cli_section, cli_success, confirm_action, output, print_table},
    state::AppState,
};

fn git_remote_url(state: &AppState, slug: &str) -> String {
    format!(
        "{}/git/{}",
        state.api_client.url("").trim_end_matches('/'),
        slug
    )
}

fn ssh_git_url(state: &AppState, slug: &str) -> String {
    let host = state
        .api_client
        .url("")
        .trim_end_matches('/')
        .trim_start_matches("http://")
        .trim_start_matches("https://")
        .split(':')
        .next()
        .unwrap_or("localhost")
        .to_string();

    format!("slasha@{}:{}.git", host, slug)
}

fn print_app(state: &AppState, app: &App) {
    cli_section(&app.name);
    cli_label("Slug", &app.slug);
    cli_label("Status", &app.status);
    cli_label("Branch", &app.default_branch);

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

pub async fn handle_list(state: &AppState) -> Result<()> {
    let body = state.api_client.get("/api/apps").await?;

    let apps: Vec<App> =
        serde_json::from_value(body["apps"].clone()).context("Failed to parse apps")?;

    output(state.output_mode, &apps, || {
        if apps.is_empty() {
            cli_info("No apps yet. Run slasha create <name> to create one.");
        } else {
            print_table(
                &["NAME", "SLUG", "STATUS", "BRANCH"],
                apps.iter()
                    .map(|a| {
                        vec![
                            a.name.clone(),
                            a.slug.clone(),
                            a.status.clone(),
                            a.default_branch.clone(),
                        ]
                    })
                    .collect(),
            );
        }
    })?;

    Ok(())
}

pub async fn handle_create(state: &AppState, name: &str) -> Result<()> {
    let body = state
        .api_client
        .post("/api/apps", &json!({ "name": name }))
        .await?;

    let app: App = serde_json::from_value(body["app"].clone()).context("Failed to parse app")?;

    output(state.output_mode, &app, || {
        cli_success("App created");
        print_app(state, &app);
    })?;

    Ok(())
}

pub async fn handle_info(state: &AppState, slug: &str) -> Result<()> {
    let body = state.api_client.get(&format!("/api/apps/{}", slug)).await?;

    let app: App = serde_json::from_value(body["app"].clone()).context("Failed to parse app")?;

    output(state.output_mode, &app, || {
        print_app(state, &app);
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

    let mut config = Config::load().unwrap_or_default();
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
