use anyhow::{Context as _, Result};
use serde_json::json;

use crate::{
    commands::{resolve::resolve_deployment_id, responses::OkResponse},
    context::Context,
    output::{cli_success, spinner},
};

pub async fn handle_scale(
    pairs: Vec<String>,
    server_override: Option<&str>,
    app_override: Option<&str>,
) -> Result<()> {
    let ctx = Context::new(server_override, app_override)?;
    let (client, slug) = ctx.require_context()?;

    let deployment_id = resolve_deployment_id(client, slug, None).await?;

    let mut scales = Vec::new();
    for pair in pairs {
        let parts: Vec<&str> = pair.split('=').collect();
        if parts.len() != 2 {
            anyhow::bail!(
                "Invalid scale format '{}'. Expected TYPE=COUNT (e.g. web=2)",
                pair
            );
        }
        let process_type = parts[0].trim().to_lowercase();
        let count: u32 = parts[1].parse().with_context(|| {
            format!(
                "Invalid count '{}' for process '{}'",
                parts[1], process_type
            )
        })?;
        scales.push((process_type, count));
    }

    for (process_type, count) in scales {
        let _spin = spinner(&format!("Scaling {} to {}...", process_type, count));

        let _: OkResponse = client
            .post(
                &format!("/api/apps/{}/deployments/{}/scale", slug, deployment_id),
                &json!({
                    "process_type": process_type,
                    "count": count
                }),
            )
            .await?;

        cli_success(format!("Scaled {} to {}.", process_type, count));
    }

    Ok(())
}
