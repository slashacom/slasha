mod app_env;
mod apps;
mod auth;
mod clap_app;
mod config;
mod deployments;
mod diagnostic;
#[cfg(feature = "serve")]
mod git_ssh;
mod http;
mod output;
mod scale;
mod service_env;
mod services;
mod ssh_keys;
mod state;
mod token;
mod users;

use clap::Parser;
use colored::Colorize;
use serde_json::json;

use crate::{
    clap_app::{ClapApp, Command},
    config::Config,
    diagnostic::DiagnosticReport,
    http::ApiClient,
    output::print_json,
    state::AppState,
};

fn resolve_app(flag: Option<String>) -> anyhow::Result<String> {
    if let Some(app) = flag {
        return Ok(app);
    }

    let config = Config::load()?;
    if let Some(slug) = config.app
        && !slug.is_empty()
    {
        return Ok(slug);
    }

    anyhow::bail!("Missing --app flag and no app specified in slasha.toml");
}

async fn run(cli: ClapApp) -> anyhow::Result<()> {
    let ClapApp {
        command,
        output_mode,
        url,
        diagnostic,
    } = cli;

    if diagnostic {
        DiagnosticReport::generate()?.print()?;
        return Ok(());
    }

    let state = AppState {
        api_client: ApiClient::from_config()?.with_url_override(url),
        output_mode,
    };

    match command {
        #[cfg(feature = "serve")]
        Command::Serve => slasha_server::start_server().await?,
        #[cfg(feature = "serve")]
        Command::GitSsh { user_id } => git_ssh::handle(user_id).await?,

        Command::Status => auth::handle_status(&state).await?,
        Command::Login => auth::handle_login(&state).await?,
        Command::Logout => auth::handle_logout(&state).await?,
        Command::Me => auth::handle_me(&state).await?,

        Command::SetUrl { url } => {
            let mut config = Config::load()?;
            config.base_url = Some(url);
            config.save()?;
            return Ok(());
        }

        Command::Version { verbose } => {
            println!("{} {}", "Version".green(), env!("CARGO_PKG_VERSION"));

            if verbose {
                println!("{} {}", "Authors".green(), env!("CARGO_PKG_AUTHORS"));
                println!("{} {}", "License".green(), env!("CARGO_PKG_LICENSE"));
                println!("{} {}", "Repository".green(), env!("CARGO_PKG_REPOSITORY"));
                println!("{} {}", "Build Timestamp".green(), env!("BUILD_TIMESTAMP"));
            }

            return Ok(());
        }

        Command::Create { name } => apps::handle_create(&state, &name).await?,
        Command::Delete { app, yes } => {
            apps::handle_delete(&state, &resolve_app(app)?, yes).await?
        }
        Command::Info { app } => apps::handle_info(&state, &resolve_app(app)?).await?,
        Command::List => apps::handle_list(&state).await?,
        Command::Link { app } => apps::handle_link(&state, app).await?,

        Command::Deploy { app, commit } => {
            deployments::handle_trigger(&state, &resolve_app(app)?, commit).await?
        }
        Command::Deployments { app, command } => {
            deployments::dispatch(&state, &resolve_app(app)?, command).await?
        }

        Command::Provision {
            app,
            kind,
            name,
            version,
        } => services::handle_create(&state, &resolve_app(app)?, &kind, &name, &version).await?,

        Command::AppEnv { app, command } => {
            app_env::dispatch(&state, &resolve_app(app)?, command).await?
        }

        Command::Services { app, command } => {
            services::dispatch(&state, &resolve_app(app)?, command).await?
        }

        Command::Scale { app, pairs } => {
            scale::handle_scale(&state, &resolve_app(app)?, pairs).await?
        }

        Command::SshKeys { command } => ssh_keys::dispatch(&state, command).await?,
        Command::Users { command } => users::dispatch(&state, command).await?,
    }

    Ok(())
}

#[tokio::main]
async fn main() {
    if std::env::var_os("NO_COLOR").is_some()
        || !std::io::IsTerminal::is_terminal(&std::io::stdout())
    {
        colored::control::set_override(false);
    }

    let cli = ClapApp::parse();
    let is_json = cli.output_mode.is_json();
    let result = run(cli).await;

    if let Err(e) = result {
        if is_json {
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
