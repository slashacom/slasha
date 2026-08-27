use anyhow::Result;
use colored::Colorize;
use serde::{Deserialize, Serialize};

use crate::{
    context::Context,
    output::{cli_label, cli_success},
};

#[derive(Deserialize, Serialize)]
struct ServicesHealth {
    database: String,
    docker: String,
}

#[derive(Deserialize, Serialize)]
struct HealthResponse {
    status: String,
    version: String,
    services: ServicesHealth,
}

pub async fn handle(server_override: Option<&str>) -> Result<()> {
    let ctx = Context::new(server_override, None)?;
    let health: HealthResponse = ctx.api_client()?.get("/api/health").await?;

    let status_colored = if health.status == "ok" {
        health.status.green()
    } else {
        health.status.red()
    };

    cli_success(format!("Server is {}", status_colored));
    cli_label("Version", &health.version);

    cli_label(
        "Database",
        if health.services.database == "ok" {
            health.services.database.green()
        } else {
            health.services.database.red()
        },
    );
    cli_label(
        "Docker",
        if health.services.docker == "ok" {
            health.services.docker.green()
        } else {
            health.services.docker.red()
        },
    );

    Ok(())
}
