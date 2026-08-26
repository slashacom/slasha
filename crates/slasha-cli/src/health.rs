use anyhow::Result;
use colored::Colorize;
use serde::{Deserialize, Serialize};

use crate::{
    context::Context,
    output::{cli_label, cli_success},
};

#[derive(Deserialize, Serialize)]
struct HealthResponse {
    status: String,
    version: String,
}

pub async fn handle(ctx: &Context) -> Result<()> {
    let health: HealthResponse = ctx.api_client.get("/api/health").await?;

    cli_success(format!("Server is {}", health.status.green()));
    cli_label("Version", &health.version);

    Ok(())
}
