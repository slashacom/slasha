mod output;
mod app_env;
mod apps;
mod auth;
mod clap_app;
mod config;
mod deployments;
mod http;
mod service_env;
mod services;
mod ssh_keys;
mod state;
mod users;

use std::fs;

use clap::Parser;
use serde_json::json;

use crate::{
    clap_app::{ClapApp, Command},
    output::print_json,
    state::AppState,
};

#[tokio::main]
async fn main() {
    let cli = ClapApp::parse();
    let json = cli.output.is_json();

    let result = run(cli).await;

    if let Err(e) = result {
        if json {
            let _ = print_json(&json!({ "error": format!("{:#}", e) }));
        } else {
            eprintln!("{}", e);
            let mut source = e.source();
            while let Some(cause) = source {
                eprintln!("  {} {}", colored::Colorize::dimmed("caused by:"), cause);
                source = cause.source();
            }
        }

        std::process::exit(1);
    }
}

fn resolve_app(flag: Option<String>) -> anyhow::Result<String> {
    if let Some(app) = flag {
        return Ok(app);
    }
    let content = fs::read_to_string(".slasha")
        .map_err(|_| anyhow::anyhow!("Missing --app flag and no .slasha file found"))?;
    let slug = content.trim().to_string();
    if slug.is_empty() {
        anyhow::bail!("Missing --app flag and .slasha file is empty");
    }
    Ok(slug)
}

async fn run(cli: ClapApp) -> anyhow::Result<()> {
    let output = cli.output;
    let state = AppState {
        client: http::client()?.with_url_override(cli.url),
        output,
    };

    match cli.command {
        Command::Status => return auth::handle_status(&state).await,
        Command::Login => return auth::handle_login(&state).await,
        Command::Me => return auth::handle_me(&state).await,
        Command::Logout => return auth::handle_logout(&state).await,
        Command::SetUrl { url } => {
            let mut conf = config::Config::load()?;
            conf.base_url = url.clone();
            conf.save()?;
            return Ok(());
        }
        Command::List => return apps::handle_list(&state).await,
        Command::Create { name } => return apps::handle_create(&state, &name).await,
        Command::Link => return apps::handle_link(&state, cli.app).await,
        Command::SshKeys { command } => return ssh_keys::dispatch(&state, command).await,
        Command::Users { command } => return users::dispatch(&state, command).await,
        _ => {}
    }

    let slug = resolve_app(cli.app)?;

    match cli.command {
        Command::Info => apps::handle_info(&state, &slug).await?,
        Command::Delete { yes } => apps::handle_delete(&state, &slug, yes).await?,
        Command::Deploy { commit } => deployments::handle_trigger(&state, &slug, commit).await?,
        Command::Deployments { command } => deployments::dispatch(&state, &slug, command).await?,
        Command::Provision {
            kind,
            name,
            version,
        } => services::handle_create(&state, &slug, &kind, &name, &version).await?,
        Command::AppEnv { command } => app_env::dispatch(&state, &slug, command).await?,
        Command::Services { command } => services::dispatch(&state, &slug, command).await?,
        _ => unreachable!(),
    }

    Ok(())
}
