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
}
