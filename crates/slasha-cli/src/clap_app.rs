use std::str::FromStr;

use clap::{
    Parser, Subcommand,
    builder::{PossibleValuesParser, TypedValueParser},
};
use slasha_db::service::ServiceKind;
use strum::VariantNames;

#[derive(Parser)]
#[command(name = "slasha", author, version)]
pub struct ClapApp {
    #[arg(
        short = 's',
        long = "server-url",
        global = true,
        help = "Target server URL"
    )]
    pub server_override: Option<String>,

    #[arg(
        short = 'a',
        long = "app",
        global = true,
        help = "Target application slug"
    )]
    pub app_override: Option<String>,

    #[command(subcommand)]
    pub command: Command,
}

#[derive(Subcommand)]
pub enum Command {
    #[cfg(feature = "serve")]
    #[command(name = "serve", hide = true)]
    Serve,

    #[cfg(feature = "serve")]
    #[command(name = "git-ssh", hide = true)]
    GitSsh { user_id: String },

    #[command(name = "health", about = "Check server health")]
    Health,

    #[command(name = "version", about = "Display version information")]
    Version,

    #[command(name = "auth", about = "Manage user authentication")]
    Auth {
        #[command(subcommand)]
        command: AuthCommand,
    },

    #[command(name = "diagnostic", about = "Generate diagnostic report")]
    Diagnostic,

    #[command(name = "completion", about = "Generate shell autocompletion script")]
    Completion {
        #[arg(value_enum, help = "Target shell")]
        shell: clap_complete::Shell,
    },

    #[command(name = "apps", about = "Manage applications")]
    Apps {
        #[command(subcommand)]
        command: AppsCommand,
    },

    #[command(name = "deploy", about = "Deploy an application")]
    Deploy {
        #[arg(long, value_name = "SHA", help = "Git commit SHA (defaults to HEAD)")]
        commit: Option<String>,
    },

    #[command(name = "logs", about = "View deployment logs")]
    Logs {
        #[arg(value_name = "ID", help = "Deployment ID (defaults to latest)")]
        deployment_id: Option<String>,
        #[arg(short = 'f', long, help = "Follow log stream")]
        follow: bool,
    },

    #[command(name = "scale", about = "Scale process instances")]
    Scale {
        #[arg(
            value_name = "TYPE=COUNT",
            required = true,
            num_args = 1..,
            help = "Process type and count (e.g. web=2 worker=1)"
        )]
        pairs: Vec<String>,
    },

    #[command(name = "deployments", about = "Manage deployments")]
    Deployments {
        #[command(subcommand)]
        command: DeploymentsCommand,
    },

    #[command(name = "services", about = "Manage services")]
    Services {
        #[command(subcommand)]
        command: ServicesCommand,
    },

    #[command(name = "env", about = "Manage environment variables")]
    Env {
        #[command(subcommand)]
        command: AppEnvCommand,
    },

    #[command(name = "domains", about = "Manage custom domains")]
    Domains {
        #[command(subcommand)]
        command: DomainsCommand,
    },

    #[command(name = "ssh-keys", about = "Manage SSH keys for Git deployment")]
    SshKeys {
        #[command(subcommand)]
        command: SshKeysCommand,
    },

    #[command(name = "link", about = "Link local directory to an application")]
    Link {},

    #[command(name = "config", about = "Manage CLI configuration")]
    Config {
        #[command(subcommand)]
        command: ConfigCommand,
    },
}

#[derive(Subcommand)]
pub enum AppsCommand {
    #[command(name = "list", about = "List applications")]
    List,

    #[command(name = "create", about = "Create an application")]
    Create {
        #[arg(help = "Application name")]
        name: String,
    },

    #[command(name = "info", about = "Display application details")]
    Info,

    #[command(name = "delete", about = "Delete an application")]
    Delete {
        #[arg(short = 'y', long, help = "Skip confirmation prompt")]
        yes: bool,
    },
}

#[derive(Subcommand)]
pub enum DomainsCommand {
    #[command(name = "list", about = "List custom domains")]
    List,

    #[command(name = "add", about = "Add a custom domain")]
    Add {
        #[arg(help = "Domain name")]
        domain: String,
    },

    #[command(name = "remove", about = "Remove a custom domain")]
    Remove {
        #[arg(help = "Domain name")]
        domain: String,
    },
}

#[derive(Subcommand)]
pub enum DeploymentsCommand {
    #[command(name = "list", about = "List deployments")]
    List,

    #[command(name = "stop", about = "Stop a deployment")]
    Stop {
        #[arg(value_name = "ID", help = "Deployment ID (defaults to latest)")]
        deployment_id: Option<String>,
    },

    #[command(name = "restart", about = "Restart a deployment")]
    Restart {
        #[arg(value_name = "ID", help = "Deployment ID (defaults to latest)")]
        deployment_id: Option<String>,
    },

    #[command(name = "redeploy", about = "Redeploy previous build")]
    Redeploy {
        #[arg(value_name = "ID", help = "Deployment ID (defaults to latest)")]
        deployment_id: Option<String>,
    },

    #[command(name = "rollback", about = "Roll back to a previous deployment")]
    Rollback {
        #[arg(value_name = "ID", help = "Target deployment ID")]
        deployment_id: Option<String>,
    },

    #[command(name = "delete", about = "Delete a deployment record")]
    Delete {
        #[arg(value_name = "ID", help = "Deployment ID")]
        deployment_id: Option<String>,
        #[arg(short = 'y', long, help = "Skip confirmation prompt")]
        yes: bool,
    },
}

#[derive(Subcommand)]
pub enum AppEnvCommand {
    #[command(name = "list", about = "List environment variables")]
    List,

    #[command(name = "set", about = "Set environment variables")]
    Set {
        #[arg(
            value_name = "KEY=VALUE",
            required = true,
            num_args = 1..,
            help = "KEY=VALUE pairs"
        )]
        pairs: Vec<String>,
    },

    #[command(name = "unset", about = "Remove environment variables")]
    Unset {
        #[arg(
            value_name = "KEY",
            required = true,
            num_args = 1..,
            help = "Environment variable keys"
        )]
        keys: Vec<String>,
    },
}

#[derive(Subcommand)]
pub enum ServicesCommand {
    #[command(name = "list", about = "List services")]
    List,

    #[command(name = "provision", about = "Provision a service")]
    Provision {
        #[arg(
            value_parser = PossibleValuesParser::new(ServiceKind::VARIANTS)
                .map(|s| {
                    ServiceKind::VARIANTS
                        .iter()
                        .find(|v| v.eq_ignore_ascii_case(&s))
                        .and_then(|v| ServiceKind::from_str(v).ok())
                        .expect("valid service kind")
                }),
            ignore_case = true,
            help = "Service type"
        )]
        kind: ServiceKind,
        #[arg(help = "Service name")]
        name: String,
        #[arg(
            short,
            long,
            help = "Service version or tag (defaults to latest supported version)"
        )]
        version: Option<String>,
    },

    #[command(name = "restart", about = "Restart a service")]
    Restart {
        #[arg(value_name = "NAME", help = "Service name")]
        service: String,
    },

    #[command(name = "redeploy", about = "Redeploy a service")]
    Redeploy {
        #[arg(value_name = "NAME", help = "Service name")]
        service: String,
    },

    #[command(name = "stop", about = "Stop a service")]
    Stop {
        #[arg(value_name = "NAME", help = "Service name")]
        service: String,
        #[arg(short = 'y', long, help = "Skip confirmation prompt")]
        yes: bool,
    },

    #[command(name = "delete", about = "Delete a service")]
    Delete {
        #[arg(value_name = "NAME", help = "Service name")]
        service: String,
        #[arg(short = 'y', long, help = "Skip confirmation prompt")]
        yes: bool,
    },

    #[command(name = "logs", about = "View service logs")]
    Logs {
        #[arg(value_name = "NAME", help = "Service name")]
        service: String,
        #[arg(short = 'f', long, help = "Follow log stream")]
        follow: bool,
    },

    #[command(name = "env", about = "Manage service environment variables")]
    Env {
        #[arg(value_name = "NAME", help = "Service name")]
        service: String,
        #[command(subcommand)]
        command: ServiceEnvCommand,
    },

    #[command(name = "backup", about = "Backup service database")]
    Backup {
        #[arg(value_name = "NAME", help = "Service name")]
        service: String,
        #[arg(
            short = 'f',
            long = "file",
            value_name = "FILE",
            help = "Output file path"
        )]
        file: Option<String>,
    },

    #[command(name = "proxy", about = "Proxy local port to service")]
    Proxy {
        #[arg(value_name = "NAME", help = "Service name")]
        service: String,
        #[arg(
            short = 'p',
            long,
            value_name = "PORT",
            help = "Local port to listen on"
        )]
        port: Option<u16>,
        #[arg(long, help = "Hide secrets in connection string")]
        no_secret: bool,
    },
}

#[derive(Subcommand)]
pub enum ServiceEnvCommand {
    #[command(name = "list", about = "List service environment variables")]
    List,

    #[command(name = "set", about = "Set service environment variables")]
    Set {
        #[arg(
            value_name = "KEY=VALUE",
            required = true,
            num_args = 1..,
            help = "KEY=VALUE pairs"
        )]
        pairs: Vec<String>,
    },

    #[command(name = "unset", about = "Remove service environment variables")]
    Unset {
        #[arg(
            value_name = "KEY",
            required = true,
            num_args = 1..,
            help = "Environment variable keys"
        )]
        keys: Vec<String>,
    },
}

#[derive(Subcommand)]
pub enum SshKeysCommand {
    #[command(name = "list", about = "List SSH public keys")]
    List,

    #[command(name = "add", about = "Add an SSH key for Git deployment")]
    Add {
        #[arg(help = "Key name")]
        name: String,
        #[arg(help = "Public key content")]
        pubkey: Option<String>,
        #[arg(short, long, value_name = "PATH", help = "Path to public key file")]
        file: Option<String>,
    },

    #[command(name = "remove", about = "Remove an SSH public key")]
    Remove {
        #[arg(help = "SSH key name")]
        name: String,
    },
}

#[derive(Subcommand)]
pub enum ConfigCommand {
    #[command(name = "set", about = "Set CLI configuration")]
    Set {
        #[arg(value_name = "KEY", help = "Configuration key")]
        key: String,
        #[arg(value_name = "VALUE", help = "Configuration value")]
        value: String,
    },

    #[command(name = "get", about = "Get CLI configuration")]
    Get {
        #[arg(value_name = "KEY", help = "Configuration key")]
        key: String,
    },
}

#[derive(Subcommand)]
pub enum AuthCommand {
    #[command(name = "login", about = "Log in to server")]
    Login,

    #[command(name = "logout", about = "Log out from server")]
    Logout,

    #[command(name = "status", about = "Show authentication status")]
    Status,
}
