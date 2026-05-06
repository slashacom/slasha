use clap::{Parser, Subcommand};

#[derive(Parser)]
#[command(author, version, about, long_about = None)]
pub struct ClapApp {
    #[command(subcommand)]
    pub command: Command,
}

#[derive(Subcommand)]
pub enum Command {
    #[command(
        name = "status",
        about = "Check the status of the slasha server",
        override_usage = "slasha status"
    )]
    Status,

    #[command(
        name = "set-url",
        about = "Set the base API URL of the remote slasha server",
        override_usage = "slasha set-url <URL>"
    )]
    SetUrl { url: String },

    #[command(name = "login", about = "Log into the slasha server")]
    Login,

    #[command(name = "me", about = "Get current user info")]
    Me,

    #[command(name = "apps", about = "Manage apps")]
    Apps {
        #[command(subcommand)]
        command: AppsCommand,
    },

    #[cfg(feature = "serve")]
    #[command(
        name = "serve",
        about = "Run the slasha server",
        override_usage = "slasha serve"
    )]
    Serve,
}

#[derive(Subcommand)]
pub enum AppsCommand {
    #[command(name = "create", about = "Create a new app")]
    Create { name: String },

    #[command(name = "delete", about = "Delete an app")]
    Delete { slug: String },

    #[command(name = "info", about = "Get info about an app")]
    Info { slug: String },

    #[command(name = "list", about = "List all your apps")]
    List,
}
