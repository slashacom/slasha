use anyhow::Result;
use colored::Colorize;
use serde::{Deserialize, Serialize};
use serde_json::json;
use slasha_db::app::{App, AppSource};

use crate::{
    clap_app::AppsCommand,
    config::ProjectConfig,
    context::Context,
    output::{cli_info, cli_label, cli_section, cli_success, confirm_action, print_table, spinner},
    resolve::resolve_slug,
};

#[derive(Deserialize, Serialize)]
pub struct AppItemResponse {
    pub app: App,
    pub runtime_status: String,
}

#[derive(Deserialize, Serialize)]
pub struct AppDetailsResponse {
    pub app: App,
    pub runtime_status: String,
}

/// Constructs the HTTPS Git remote URL for an application slug.
///
/// # Arguments
///
/// * `ctx` - Execution context ([`Context`]).
/// * `slug` - Application slug.
///
/// # Returns
///
/// The HTTPS Git repository URL string.
fn git_remote_url(ctx: &Context, slug: &str) -> String {
    format!("{}/git/{}", ctx.api_client.base_url(), slug)
}

/// Constructs the SSH Git remote URL for an application slug.
///
/// # Arguments
///
/// * `ctx` - Execution context ([`Context`]).
/// * `slug` - Application slug.
///
/// # Returns
///
/// The SSH Git repository URL string.
fn ssh_git_url(ctx: &Context, slug: &str) -> String {
    format!("slasha@{}:{}.git", ctx.api_client.git_host(), slug)
}

/// Formats and prints comprehensive application details and deployment instructions.
///
/// # Arguments
///
/// * `ctx` - Execution context ([`Context`]).
/// * `app` - Target application ([`App`]).
/// * `status` - Runtime status string slice.
fn print_app(ctx: &Context, app: &App, status: &str) {
    cli_section(&app.name);
    cli_label("Slug", &app.slug);
    cli_label("Status", status);
    cli_label("Branch", &app.default_branch);
    if !app.root_dir.is_empty() {
        cli_label("Root Dir", &app.root_dir);
    }
    cli_label("Source", app.source.to_string());

    match app.source {
        AppSource::Local => {
            cli_section("Git remotes");
            cli_label("HTTPS", git_remote_url(ctx, &app.slug));
            cli_label("SSH", ssh_git_url(ctx, &app.slug));

            cli_section("Deploy");
            cli_info(format!(
                "  git remote add slasha {}",
                ssh_git_url(ctx, &app.slug)
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

pub async fn dispatch(ctx: &Context, cmd: AppsCommand) -> Result<()> {
    match cmd {
        AppsCommand::List => handle_list(ctx).await,
        AppsCommand::Create { name } => handle_create(ctx, &name).await,
        AppsCommand::Info { slug } => handle_info(ctx, slug).await,
        AppsCommand::Delete { slug, yes } => handle_delete(ctx, slug, yes).await,
        AppsCommand::Link { slug } => handle_link(ctx, slug).await,
    }
}

#[derive(Deserialize, Serialize)]
pub struct AppListResponse {
    pub apps: Vec<AppItemResponse>,
}

pub async fn handle_list(ctx: &Context) -> Result<()> {
    let res: AppListResponse = ctx.api_client.get("/api/apps").await?;

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

pub async fn handle_create(ctx: &Context, name: &str) -> Result<()> {
    let _spin = spinner("Creating app...");

    let res: AppDetailsResponse = ctx
        .api_client
        .post("/api/apps", &json!({ "name": name, "source": "local" }))
        .await?;

    cli_success("App created.");
    print_app(ctx, &res.app, &res.runtime_status);

    Ok(())
}

pub async fn handle_info(ctx: &Context, slug_arg: Option<String>) -> Result<()> {
    let slug = resolve_slug(slug_arg)?;
    let res: AppDetailsResponse = ctx.api_client.get(&format!("/api/apps/{}", slug)).await?;

    print_app(ctx, &res.app, &res.runtime_status);

    Ok(())
}

#[derive(Deserialize, Serialize)]
pub struct AppDeleteResponse {
    pub ok: bool,
}

pub async fn handle_delete(ctx: &Context, slug_arg: Option<String>, yes: bool) -> Result<()> {
    let slug = resolve_slug(slug_arg)?;

    if !confirm_action(
        yes,
        &format!(
            "Delete app {}? This removes all deployments and services.",
            slug.red()
        ),
    )? {
        return Ok(());
    }

    let _spin = spinner("Deleting app...");
    let _: AppDeleteResponse = ctx
        .api_client
        .delete(&format!("/api/apps/{}", slug))
        .await?;

    cli_success(format!("App {} deleted.", slug));

    Ok(())
}

pub async fn handle_link(ctx: &Context, slug_arg: Option<String>) -> Result<()> {
    let slug = match slug_arg {
        Some(s) => s,
        None => {
            let res: AppListResponse = ctx.api_client.get("/api/apps").await?;

            if res.apps.is_empty() {
                anyhow::bail!("No apps found. Create one first with `slasha apps create <name>`.");
            }

            let choices: Vec<String> = res.apps.into_iter().map(|a| a.app.slug).collect();
            inquire::Select::new("Select app to link:", choices).prompt()?
        }
    };

    // make sure the app exists
    let _: AppDetailsResponse = ctx.api_client.get(&format!("/api/apps/{}", slug)).await?;

    let mut config = ProjectConfig::load()?;
    config.app = Some(slug.clone());
    config.save()?;

    cli_success(format!(
        "Linked current directory to app '{}' in slasha.toml",
        slug
    ));

    Ok(())
}
