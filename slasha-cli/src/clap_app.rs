use clap::{Parser, Subcommand, builder::PossibleValuesParser};
use slasha_db::service::ServiceKind;
use strum::VariantNames;

use crate::output::OutputMode;

#[derive(Parser)]
#[command(
    name = "slasha",
    author,
    version,
    about = "Deploy and manage apps on your Slasha PaaS"
)]
pub struct ClapApp {
    #[arg(name = "output", long, global = true, default_value = "human")]
    pub output_mode: OutputMode,

    #[arg(long, global = true, value_name = "URL")]
    pub url: Option<String>,

    #[arg(
        long,
        global = true,
        help = "Show diagnostic information for bug reports"
    )]
    pub diagnostic: bool,

    #[command(subcommand)]
    pub command: Option<Command>,
}

#[derive(Subcommand)]
pub enum Command {
    #[cfg(feature = "serve")]
    #[command(name = "serve", about = "Run the Slasha server")]
    Serve,

    #[cfg(feature = "serve")]
    #[command(name = "git-ssh", hide = true)]
    GitSsh { user_id: String },

    #[command(name = "status", about = "Check server health")]
    Status,

    #[command(name = "version", about = "Print version information")]
    Version {
        #[arg(
            long = "verbose",
            short = 'v',
            help = "show verbose version information"
        )]
        verbose: bool,
    },

    #[command(name = "login", about = "Authenticate")]
    Login,

    #[command(name = "logout", about = "Remove stored token")]
    Logout,

    #[command(name = "me", about = "Show current user")]
    Me,

    #[command(name = "ssh-keys", about = "Manage SSH keys")]
    SshKeys {
        #[command(subcommand)]
        command: SshKeysCommand,
    },

    #[command(name = "users", about = "Manage users")]
    Users {
        #[command(subcommand)]
        command: UsersCommand,
    },

    #[command(name = "create", about = "Create a new app")]
    Create { name: String },

    #[command(name = "delete", about = "Delete an app")]
    Delete {
        #[arg(long, value_name = "SLUG")]
        app: Option<String>,
        #[arg(short = 'y', long)]
        yes: bool,
    },

    #[command(name = "info", about = "Show app details")]
    Info {
        #[arg(long, value_name = "SLUG")]
        app: Option<String>,
    },

    #[command(name = "list", about = "List all apps")]
    List,

    #[command(name = "link", about = "Write app context to .slasha in cwd")]
    Link {
        #[arg(long, value_name = "SLUG")]
        app: Option<String>,
    },

    #[command(name = "set-url", about = "Persist base URL to config")]
    SetUrl { url: String },

    #[command(name = "deploy", about = "Trigger a deployment")]
    Deploy {
        #[arg(long, value_name = "SLUG")]
        app: Option<String>,
        #[arg(long, value_name = "SHA")]
        commit: Option<String>,
    },

    #[command(name = "deployments", about = "Manage deployments")]
    Deployments {
        #[arg(long, value_name = "SLUG")]
        app: Option<String>,
        #[command(subcommand)]
        command: DeploymentsCommand,
    },

    #[command(
        name = "provision",
        about = "Provision a new service (e.g. PostgreSQL, MySQL, MongoDB, Redis)"
    )]
    Provision {
        #[arg(long, value_name = "SLUG")]
        app: Option<String>,
        #[arg(long, value_parser = PossibleValuesParser::new(ServiceKind::VARIANTS))]
        kind: ServiceKind,
        #[arg(long)]
        name: String,
        #[arg(long)]
        version: String,
    },

    #[command(name = "services", about = "Manage attached services")]
    Services {
        #[arg(long, value_name = "SLUG")]
        app: Option<String>,
        #[command(subcommand)]
        command: ServicesCommand,
    },

    #[command(name = "env", about = "Manage app env vars")]
    AppEnv {
        #[arg(long, value_name = "SLUG")]
        app: Option<String>,
        #[command(subcommand)]
        command: AppEnvCommand,
    },

    #[command(name = "scale", about = "Scale process types (web=2 worker=1 ...)")]
    Scale {
        #[arg(long, value_name = "SLUG")]
        app: Option<String>,
        #[arg(value_name = "TYPE=COUNT", required = true, num_args = 1..)]
        pairs: Vec<String>,
    },
}

#[derive(Subcommand)]
pub enum DeploymentsCommand {
    #[command(name = "list", about = "List deployments")]
    List,

    #[command(name = "stop", about = "Stop a deployment")]
    Stop {
        #[arg(long, value_name = "ID")]
        deployment_id: Option<String>,
    },

    #[command(name = "restart", about = "Restart a deployment")]
    Restart {
        #[arg(long, value_name = "ID")]
        deployment_id: Option<String>,
    },

    #[command(name = "redeploy", about = "Redeploy a deployment")]
    Redeploy {
        #[arg(long, value_name = "ID")]
        deployment_id: Option<String>,
    },

    #[command(name = "delete", about = "Delete a deployment")]
    Delete {
        #[arg(long, value_name = "ID")]
        deployment_id: Option<String>,
        #[arg(short = 'y', long)]
        yes: bool,
    },

    #[command(name = "logs", about = "Stream deployment logs")]
    Logs {
        #[arg(long, value_name = "ID")]
        deployment_id: Option<String>,
        #[arg(long)]
        follow: bool,
    },
}

#[derive(Subcommand)]
pub enum AppEnvCommand {
    #[command(name = "list", about = "List all env vars for an app")]
    List,

    #[command(name = "set", about = "Set one or more env vars (KEY=VALUE ...)")]
    Set {
        #[arg(value_name = "KEY=VALUE", required = true, num_args = 1..)]
        pairs: Vec<String>,
    },

    #[command(name = "unset", about = "Remove one or more env vars")]
    Unset {
        #[arg(value_name = "KEY", required = true, num_args = 1..)]
        keys: Vec<String>,
    },
}

#[derive(Subcommand)]
pub enum ServicesCommand {
    #[command(name = "list", about = "List services attached to an app")]
    List,

    #[command(name = "stop", about = "Stop a running service")]
    Stop {
        #[arg(value_name = "NAME_OR_ID")]
        service: String,
        #[arg(short = 'y', long)]
        yes: bool,
    },

    #[command(name = "delete", about = "Delete a stopped or failed service")]
    Delete {
        #[arg(value_name = "NAME_OR_ID")]
        service: String,
        #[arg(short = 'y', long)]
        yes: bool,
    },

    #[command(name = "logs", about = "Stream service logs")]
    Logs {
        #[arg(value_name = "NAME_OR_ID")]
        service: String,
        #[arg(long)]
        follow: bool,
    },

    #[command(name = "env", about = "Manage service environment variables")]
    Env {
        #[arg(value_name = "NAME_OR_ID")]
        service: String,
        #[command(subcommand)]
        command: ServiceEnvCommand,
    },
}

#[derive(Subcommand)]
pub enum ServiceEnvCommand {
    #[command(name = "list", about = "List env vars for a service")]
    List,

    #[command(name = "set", about = "Set env vars for a service (KEY=VALUE ...)")]
    Set {
        #[arg(value_name = "KEY=VALUE", required = true, num_args = 1..)]
        pairs: Vec<String>,
    },

    #[command(name = "unset", about = "Remove env vars from a service")]
    Unset {
        #[arg(value_name = "KEY", required = true, num_args = 1..)]
        keys: Vec<String>,
    },
}

#[derive(Subcommand)]
pub enum SshKeysCommand {
    #[command(name = "list", about = "List SSH public keys")]
    List,

    #[command(name = "add", about = "Add an SSH public key")]
    Add {
        #[arg(long, value_name = "PATH", help = "Read public key from file")]
        file: Option<String>,
        #[arg(long)]
        title: Option<String>,
        #[arg(help = "Public key string (alternative to --file)")]
        pubkey: Option<String>,
    },

    #[command(name = "remove", about = "Remove an SSH public key by ID")]
    Remove { id: String },
}

#[derive(Subcommand)]
pub enum UsersCommand {
    #[command(name = "list", about = "List all users")]
    List,

    #[command(name = "create", about = "Create a new user")]
    Create {
        #[arg(long)]
        email: String,
        #[arg(
            long,
            help = "Read password from stdin instead of prompting (SLASHA_PASSWORD env is also honored)"
        )]
        password_stdin: bool,
        #[arg(long)]
        role: String,
    },

    #[command(name = "update", about = "Update a user")]
    Update {
        id: String,
        #[arg(long)]
        email: Option<String>,
        #[arg(long)]
        role: Option<String>,
    },

    #[command(name = "delete", about = "Delete a user")]
    Delete {
        id: String,
        #[arg(short = 'y', long)]
        yes: bool,
    },
}
