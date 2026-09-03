mod clap_app;
mod commands;
mod config;
mod context;
mod diagnostic;
mod http;
mod output;
mod token;

use clap::Parser;
use clap_app::ClapApp;
use colored::Colorize;

#[tokio::main]
async fn main() {
    if std::env::var_os("NO_COLOR").is_some() {
        colored::control::set_override(false);
    }

    let cli = ClapApp::parse();

    if let Err(e) = commands::execute(cli).await {
        eprintln!("{e}");

        let mut source = e.source();
        while let Some(cause) = source {
            eprintln!("{} {cause}", "caused by:".dimmed());
            source = cause.source();
        }

        std::process::exit(1);
    }
}
