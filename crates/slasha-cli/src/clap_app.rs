use clap::{Parser, Subcommand, builder::PossibleValuesParser};
use slasha_db::service::ServiceKind;
use strum::VariantNames;

#[derive(Parser)]
#[command(name = "slasha", author, version)]
pub struct ClapApp {
    #[arg(
        short = 's',
        long = "server-url",
        alias = "server",
        global = true,
        help = "Override target Slasha server base URL"
    )]
    pub server_override: Option<String>,

    #[arg(
        short = 'a',
        long = "app",
        global = true,
        help = "Override target application slug"
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

    #[command(name = "health", about = "Print server health")]
    Health,

    #[command(
        name = "version",
        about = "Print CLI binary build version, git commit, and target platform"
    )]
    Version,

    #[command(
        name = "auth",
        about = "Re-authenticate or display user profile for the linked Slasha server"
    )]
    Auth {
        #[command(subcommand)]
        command: AuthCommand,
    },

    #[command(
        name = "diagnostic",
        about = "Generate Markdown report of local system, compiler, and CLI telemetry"
    )]
    Diagnostic,

    #[command(name = "apps", about = "Manage applications")]
    Apps {
        #[command(subcommand)]
        command: AppsCommand,
    },

    #[command(
        name = "deploy",
        about = "Trigger a build and rolling release for an application"
    )]
    Deploy {
        #[arg(long, value_name = "SHA")]
        commit: Option<String>,
    },

    #[command(
        name = "logs",
        about = "Fetch static logs or stream live stdout/stderr from deployment processes"
    )]
    Logs {
        #[arg(
            long = "deployment",
            value_name = "ID",
            help = "Deployment ID (defaults to latest)"
        )]
        deployment_id: Option<String>,
        #[arg(long)]
        follow: bool,
    },

    #[command(
        name = "scale",
        about = "Update process container counts for an application (e.g. web=2 worker=1)"
    )]
    Scale {
        #[arg(value_name = "TYPE=COUNT", required = true, num_args = 1..)]
        pairs: Vec<String>,
    },

    #[command(
        name = "deployments",
        about = "Manage deployment history, inspect build logs, restart, redeploy, or rollback releases"
    )]
    Deployments {
        #[command(subcommand)]
        command: DeploymentsCommand,
    },

    #[command(
        name = "services",
        about = "Manage attached datastore services (PostgreSQL, MySQL, Redis, MongoDB), proxies, and backups"
    )]
    Services {
        #[command(subcommand)]
        command: ServicesCommand,
    },

    #[command(
        name = "env",
        about = "Read, set, or unset environment variables for an application"
    )]
    Env {
        #[command(subcommand)]
        command: AppEnvCommand,
    },

    #[command(
        name = "domains",
        about = "Attach or remove custom HTTP domain names routed to an application"
    )]
    Domains {
        #[command(subcommand)]
        command: DomainsCommand,
    },

    #[command(
        name = "ssh-keys",
        about = "Manage public SSH keys authorized for Git deployment authentication"
    )]
    SshKeys {
        #[command(subcommand)]
        command: SshKeysCommand,
    },

    #[command(
        name = "link",
        about = "Link current working directory to a Slasha server and application in .slasha/config.toml"
    )]
    Link {},

    #[command(name = "config", about = "Manage global CLI configuration")]
    Config {
        #[command(subcommand)]
        command: ConfigCommand,
    },
}

#[derive(Subcommand)]
pub enum AppsCommand {
    #[command(
        name = "list",
        about = "List all applications with runtime status, branch, and source"
    )]
    List,

    #[command(
        name = "create",
        about = "Create a new application and initialize HTTP and SSH Git deployment endpoints"
    )]
    Create { name: String },

    #[command(
        name = "info",
        about = "Display detailed application metadata, source config, and Git remote URLs"
    )]
    Info,

    #[command(
        name = "delete",
        about = "Irreversibly delete an application, its deployment history, environment vars, and attached services"
    )]
    Delete {
        #[arg(short = 'y', long)]
        yes: bool,
    },
}

#[derive(Subcommand)]
pub enum DomainsCommand {
    #[command(
        name = "list",
        about = "List custom domain names attached to an application"
    )]
    List,

    #[command(
        name = "add",
        about = "Attach a custom domain name to route HTTP traffic to an application"
    )]
    Add { domain: String },

    #[command(
        name = "remove",
        about = "Detach a custom domain name from an application without destroying app compute or data"
    )]
    Remove { domain: String },
}

#[derive(Subcommand)]
pub enum DeploymentsCommand {
    #[command(
        name = "list",
        about = "List deployment history and status for an application"
    )]
    List,

    #[command(
        name = "stop",
        about = "Stop running containers for a deployment without deleting the deployment record"
    )]
    Stop {
        #[arg(value_name = "ID", help = "Deployment ID (defaults to latest)")]
        deployment_id: Option<String>,
    },

    #[command(
        name = "restart",
        about = "Reboot container processes for a deployment without rebuilding image or code"
    )]
    Restart {
        #[arg(value_name = "ID", help = "Deployment ID (defaults to latest)")]
        deployment_id: Option<String>,
    },

    #[command(
        name = "redeploy",
        about = "Re-trigger build pipeline for a deployment to recreate containers from stored commit"
    )]
    Redeploy {
        #[arg(value_name = "ID", help = "Deployment ID (defaults to latest)")]
        deployment_id: Option<String>,
    },

    #[command(
        name = "rollback",
        about = "Revert live application containers to a previously built successful deployment image"
    )]
    Rollback {
        #[arg(value_name = "ID", help = "Deployment ID")]
        deployment_id: Option<String>,
    },

    #[command(
        name = "delete",
        about = "Permanently remove a deployment record from history"
    )]
    Delete {
        #[arg(value_name = "ID", help = "Deployment ID (defaults to latest)")]
        deployment_id: Option<String>,
        #[arg(short = 'y', long)]
        yes: bool,
    },
}

#[derive(Subcommand)]
pub enum AppEnvCommand {
    #[command(
        name = "list",
        about = "List all custom environment variables configured for an application"
    )]
    List,

    #[command(
        name = "set",
        about = "Set environment variables (KEY=VALUE ...) and restart deployment containers to apply"
    )]
    Set {
        #[arg(value_name = "KEY=VALUE", required = true, num_args = 1..)]
        pairs: Vec<String>,
    },

    #[command(
        name = "unset",
        about = "Remove environment variables and restart deployment containers to apply"
    )]
    Unset {
        #[arg(value_name = "KEY", required = true, num_args = 1..)]
        keys: Vec<String>,
    },
}

#[derive(Subcommand)]
pub enum ServicesCommand {
    #[command(
        name = "list",
        about = "List all datastore services attached to an application"
    )]
    List,

    #[command(
        name = "provision",
        about = "Provision a new managed datastore service (PostgreSQL, MySQL, Redis, MongoDB) for an application"
    )]
    Provision {
        #[arg(long, value_parser = PossibleValuesParser::new(ServiceKind::VARIANTS))]
        kind: ServiceKind,
        #[arg(long)]
        name: String,
        #[arg(long)]
        version: String,
    },

    #[command(
        name = "restart",
        about = "Reboot service container process without modifying persistent volume data"
    )]
    Restart {
        #[arg(value_name = "NAME")]
        service: String,
    },

    #[command(
        name = "redeploy",
        about = "Reprovision service container using stored image and env without wiping data volume"
    )]
    Redeploy {
        #[arg(value_name = "NAME")]
        service: String,
    },

    #[command(
        name = "stop",
        about = "Stop running service container to free compute resources while preserving volume data"
    )]
    Stop {
        #[arg(value_name = "NAME")]
        service: String,
        #[arg(short = 'y', long)]
        yes: bool,
    },

    #[command(
        name = "delete",
        about = "Irreversibly delete a service container and erase its persistent database volume"
    )]
    Delete {
        #[arg(value_name = "NAME")]
        service: String,
        #[arg(short = 'y', long)]
        yes: bool,
    },

    #[command(
        name = "logs",
        about = "Fetch or stream live logs from a service container process"
    )]
    Logs {
        #[arg(value_name = "NAME")]
        service: String,
        #[arg(long)]
        follow: bool,
    },

    #[command(
        name = "env",
        about = "Read or update environment variables for a service instance"
    )]
    Env {
        #[arg(value_name = "NAME")]
        service: String,
        #[command(subcommand)]
        command: ServiceEnvCommand,
    },

    #[command(
        name = "backup",
        about = "Stream database dump from a service container to stdout or save to a file"
    )]
    Backup {
        #[arg(value_name = "NAME")]
        service: String,
        #[arg(
            short = 'f',
            long = "file",
            value_name = "FILE",
            help = "Write dump to file instead of stdout"
        )]
        file: Option<String>,
    },

    #[command(
        name = "proxy",
        about = "Open a local TCP listener that tunnels connections over WebSocket to a service container"
    )]
    Proxy {
        #[arg(value_name = "NAME")]
        service: String,
        #[arg(short = 'p', long, value_name = "PORT")]
        port: Option<u16>,
        #[arg(long, help = "Mask passwords in printed connection string")]
        no_secret: bool,
    },
}

#[derive(Subcommand)]
pub enum ServiceEnvCommand {
    #[command(
        name = "list",
        about = "List custom environment variables configured for a service instance"
    )]
    List,

    #[command(
        name = "set",
        about = "Set environment variables (KEY=VALUE ...) for a service and restart its container"
    )]
    Set {
        #[arg(value_name = "KEY=VALUE", required = true, num_args = 1..)]
        pairs: Vec<String>,
    },

    #[command(
        name = "unset",
        about = "Remove environment variables from a service and restart its container"
    )]
    Unset {
        #[arg(value_name = "KEY", required = true, num_args = 1..)]
        keys: Vec<String>,
    },
}

#[derive(Subcommand)]
pub enum SshKeysCommand {
    #[command(
        name = "list",
        about = "List all SSH public keys registered for the authenticated user"
    )]
    List,

    #[command(
        name = "add",
        about = "Register a new SSH public key from file (--file) or string for Git push authentication"
    )]
    Add {
        #[arg(long, value_name = "PATH", help = "Read public key from file")]
        file: Option<String>,
        #[arg(long)]
        title: Option<String>,
        #[arg(help = "Public key string (alternative to --file)")]
        pubkey: Option<String>,
    },

    #[command(
        name = "remove",
        about = "Revoke and delete a registered SSH public key by ID, preventing future Git SSH pushes"
    )]
    Remove { id: String },
}

#[derive(Subcommand)]
pub enum ConfigCommand {
    #[command(name = "set", about = "Set configuration key in global config")]
    Set {
        #[arg(value_name = "KEY", help = "Key")]
        key: String,
        #[arg(value_name = "VALUE", help = "Value")]
        value: String,
    },

    #[command(name = "get", about = "Get configuration value from global config")]
    Get {
        #[arg(value_name = "KEY", help = "Key")]
        key: String,
    },
}

#[derive(Subcommand)]
pub enum AuthCommand {
    #[command(
        name = "login",
        about = "Authenticate CLI session interactively and store bearer token in local OS keyring"
    )]
    Login,

    #[command(
        name = "logout",
        about = "Clear the stored authentication token for a server from the OS keyring"
    )]
    Logout,

    #[command(
        name = "status",
        about = "Display profile info and role for the currently authenticated user"
    )]
    Status,
}
