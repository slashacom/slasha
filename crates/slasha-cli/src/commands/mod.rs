use crate::{
    clap_app::{ClapApp, Command},
    diagnostic::DiagnosticReport,
    output::cli_label,
};

pub mod app_env;
pub mod apps;
pub mod auth;
pub mod config;
pub mod deployments;
pub mod domains;
#[cfg(feature = "serve")]
pub mod git_ssh;
pub mod health;
pub mod link;
pub mod proxy;
pub mod responses;
pub mod scale;
pub mod service_env;
pub mod services;
pub mod ssh_keys;

pub async fn execute(clap_app: ClapApp) -> anyhow::Result<()> {
    let server_override = clap_app.server_override.as_deref();
    let app_override = clap_app.app_override.as_deref();

    match clap_app.command {
        #[cfg(feature = "serve")]
        Command::Serve => slasha_server::serve().await?,
        #[cfg(feature = "serve")]
        Command::GitSsh { user_id } => git_ssh::handle(user_id).await?,

        Command::Health => health::handle(server_override).await?,
        Command::Auth { command } => auth::dispatch(command, server_override).await?,
        Command::Config { command } => config::dispatch(command).await?,

        Command::Diagnostic => DiagnosticReport::generate()?.print()?,

        Command::Version => {
            cli_label("Version", env!("CARGO_PKG_VERSION"));
            cli_label("Authors", env!("CARGO_PKG_AUTHORS"));
            cli_label("License", env!("CARGO_PKG_LICENSE"));
            cli_label("Repository", env!("CARGO_PKG_REPOSITORY"));
            cli_label("Build Timestamp", env!("BUILD_TIMESTAMP"));
        }

        Command::Link {} => link::handle_link(app_override, server_override).await?,

        Command::Apps { command } => apps::dispatch(command, server_override, app_override).await?,
        Command::Deploy { commit } => {
            deployments::handle_trigger(commit, server_override, app_override).await?
        }
        Command::Logs {
            deployment_id,
            follow,
        } => deployments::handle_logs(deployment_id, follow, server_override, app_override).await?,
        Command::Scale { pairs } => {
            scale::handle_scale(pairs, server_override, app_override).await?
        }
        Command::Deployments { command } => {
            deployments::dispatch(command, server_override, app_override).await?
        }
        Command::Services { command } => {
            services::dispatch(command, server_override, app_override).await?
        }
        Command::Env { command } => {
            app_env::dispatch(command, server_override, app_override).await?
        }
        Command::Domains { command } => {
            domains::dispatch(command, server_override, app_override).await?
        }
        Command::SshKeys { command } => ssh_keys::dispatch(command, server_override).await?,
    }

    Ok(())
}
