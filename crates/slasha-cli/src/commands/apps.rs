use anyhow::Result;
use colored::Colorize;
use serde::{Deserialize, Serialize};
use serde_json::json;
use slasha_db::app::{App, AppSource};

use crate::{
    clap_app::AppsCommand,
    commands::responses::OkResponse,
    context::Context,
    http::ApiClient,
    output::{cli_info, cli_label, cli_section, cli_success, confirm_action, print_table, spinner},
};

#[derive(Deserialize, Serialize)]
pub struct AppItemResponse {
    pub app: App,
    pub runtime_status: String,
    pub url: Option<String>,
}

#[derive(Deserialize, Serialize)]
pub struct AppListResponse {
    pub apps: Vec<AppItemResponse>,
}

fn default_idle_status() -> String {
    "idle".to_string()
}

#[derive(Deserialize, Serialize)]
pub struct AppDetailsResponse {
    pub app: App,
    #[serde(default = "default_idle_status")]
    pub runtime_status: String,
    pub url: Option<String>,
}

pub async fn dispatch(
    cmd: AppsCommand,
    server_override: Option<&str>,
    app_override: Option<&str>,
) -> Result<()> {
    let ctx = Context::new(server_override, app_override)?;
    let client = ctx.api_client()?;

    match cmd {
        AppsCommand::List => handle_list(client).await,
        AppsCommand::Create { name } => handle_create(client, &name).await,
        AppsCommand::Info => handle_info(client, ctx.app()?).await,
        AppsCommand::Delete { yes } => handle_delete(client, ctx.app()?, yes).await,
    }
}

async fn handle_list(client: &ApiClient) -> Result<()> {
    let res: AppListResponse = client.get("/api/apps").await?;

    if res.apps.is_empty() {
        cli_info("No apps yet. Run `slasha apps create <name>` to create one.");
    } else {
        let mut rows = Vec::new();
        for item in res.apps {
            rows.push(vec![
                item.app.name,
                item.app.slug,
                item.runtime_status,
                item.app.default_branch,
                item.app.source.to_string(),
                if item.app.root_dir.is_empty() {
                    "/".to_string()
                } else {
                    item.app.root_dir
                },
            ]);
        }
        print_table(
            &["NAME", "SLUG", "STATUS", "BRANCH", "SOURCE", "ROOT DIR"],
            rows,
        );
    }

    Ok(())
}

async fn handle_create(client: &ApiClient, name: &str) -> Result<()> {
    let _spin = spinner("Creating app...");

    let res: AppDetailsResponse = client
        .post("/api/apps", &json!({ "name": name, "source": "local" }))
        .await?;

    cli_success("App created.");
    print_app(client, &res.app, &res.runtime_status, res.url.as_deref());

    Ok(())
}

async fn handle_info(client: &ApiClient, app_slug: &str) -> Result<()> {
    let res: AppDetailsResponse = client.get(&format!("/api/apps/{}", app_slug)).await?;

    print_app(client, &res.app, &res.runtime_status, res.url.as_deref());

    Ok(())
}

async fn handle_delete(client: &ApiClient, app_slug: &str, yes: bool) -> Result<()> {
    if !confirm_action(
        yes,
        &format!(
            "Delete app {}? This removes all deployments and services.",
            app_slug.red()
        ),
    )? {
        return Ok(());
    }

    let _spin = spinner("Deleting app...");
    let _: OkResponse = client.delete(&format!("/api/apps/{}", app_slug)).await?;

    cli_success(format!("App {} deleted.", app_slug));

    Ok(())
}

/// Constructs the HTTPS Git remote URL for an application slug.
fn git_remote_url(client: &ApiClient, slug: &str) -> String {
    format!("{}/git/{}", client.base_url(), slug)
}

/// Constructs the SSH Git remote URL for an application slug.
fn ssh_git_url(client: &ApiClient, slug: &str) -> String {
    format!("slasha@{}:{}.git", client.git_host(), slug)
}

fn print_app(client: &ApiClient, app: &App, status: &str, url: Option<&str>) {
    cli_section(&app.name);
    cli_label("Slug", &app.slug);
    cli_label("Status", status);
    if let Some(url) = url.filter(|u| !u.trim().is_empty()) {
        cli_label("URL", url);
    }
    cli_label("Branch", &app.default_branch);
    if !app.root_dir.is_empty() {
        cli_label("Root Dir", &app.root_dir);
    }
    cli_label("Source", app.source.to_string());

    match app.source {
        AppSource::Local => {
            cli_section("Git remotes");
            cli_label("HTTPS", git_remote_url(client, &app.slug));
            cli_label("SSH", ssh_git_url(client, &app.slug));

            cli_section("Deploy");
            cli_info(format!(
                "  git remote add slasha {}",
                ssh_git_url(client, &app.slug)
            ));
            cli_info("  git push slasha main".to_string());
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
