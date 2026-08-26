mod app_env;
mod apps;
mod auth;
mod clap_app;
mod config;
mod context;
mod deployments;
mod diagnostic;
mod domains;
#[cfg(feature = "serve")]
mod git_ssh;
mod http;
mod output;
mod proxy;
mod resolve;
mod scale;
mod service_env;
mod services;
mod ssh_keys;
mod health;
mod token;

use clap::Parser;
use colored::Colorize;

use crate::{
    clap_app::{ClapApp, Command},
    context::Context,
    diagnostic::DiagnosticReport,
    http::ApiClient,
};

async fn run(cli: ClapApp) -> anyhow::Result<()> {
    let ClapApp {
        command,
        server_url,
    } = cli;

    let ctx = Context {
        api_client: ApiClient::from_config()?.with_url_override(server_url),
    };

    match command {
        #[cfg(feature = "serve")]
        Command::Serve => slasha_server::serve().await?,
        #[cfg(feature = "serve")]
        Command::GitSsh { user_id } => git_ssh::handle(user_id).await?,

        Command::Health => health::handle(&ctx).await?,
        Command::Login => auth::handle_login(&ctx).await?,
        Command::Logout => auth::handle_logout().await?,
        Command::Me => auth::handle_me(&ctx).await?,

        Command::Config { command } => config::dispatch(command).await?,
        Command::Diagnostic => {
            DiagnosticReport::generate()?.print()?;
            return Ok(());
        }

        Command::Version => {
            println!("{} {}", "Version".green(), env!("CARGO_PKG_VERSION"));
            println!("{} {}", "Authors".green(), env!("CARGO_PKG_AUTHORS"));
            println!("{} {}", "License".green(), env!("CARGO_PKG_LICENSE"));
            println!("{} {}", "Repository".green(), env!("CARGO_PKG_REPOSITORY"));
            println!("{} {}", "Build Timestamp".green(), env!("BUILD_TIMESTAMP"));
            return Ok(());
        }

        Command::Apps { command } => apps::dispatch(&ctx, command).await?,
        Command::Deploy { slug, commit } => deployments::handle_trigger(&ctx, slug, commit).await?,
        Command::Logs {
            slug,
            deployment_id,
            follow,
        } => {
            let app_slug = resolve::resolve_slug(slug)?;
            deployments::handle_logs(&ctx, &app_slug, deployment_id, follow).await?
        }
        Command::Scale { slug, pairs } => scale::handle_scale(&ctx, slug, pairs).await?,

        Command::Deployments { slug, command } => {
            deployments::dispatch(&ctx, slug, command).await?
        }
        Command::Services { slug, command } => services::dispatch(&ctx, slug, command).await?,
        Command::Env { slug, command } => app_env::dispatch(&ctx, slug, command).await?,
        Command::Domains { slug, command } => domains::dispatch(&ctx, slug, command).await?,
        Command::SshKeys { command } => ssh_keys::dispatch(&ctx, command).await?,
    }

    Ok(())
}

#[tokio::main]
async fn main() {
    if std::env::var_os("NO_COLOR").is_some() {
        colored::control::set_override(false);
    }

    let cli = ClapApp::parse();

    match run(cli).await {
        Ok(_) => std::process::exit(0),
        Err(e) => {
            eprintln!("{}", e);
            let mut source = e.source();
            while let Some(cause) = source {
                eprintln!("  {} {}", colored::Colorize::dimmed("caused by:"), cause);
                source = cause.source();
            }
            std::process::exit(1);
        }
    }
}
