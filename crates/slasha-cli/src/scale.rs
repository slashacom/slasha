use anyhow::{Context as _, Result};
use serde_json::json;

use crate::{
    context::Context,
    deployments::OkResponse,
    output::{cli_success, spinner},
    resolve::{resolve_deployment_id, resolve_slug},
};

pub async fn handle_scale(
    ctx: &Context,
    slug_arg: Option<String>,
    pairs: Vec<String>,
) -> Result<()> {
    let slug = resolve_slug(slug_arg)?;
    let deployment_id = resolve_deployment_id(&ctx.api_client, &slug, None).await?;

    let mut scales = Vec::new();
    for pair in pairs {
        let parts: Vec<&str> = pair.split('=').collect();
        if parts.len() != 2 {
            anyhow::bail!(
                "Invalid scale format '{}'. Expected TYPE=COUNT (e.g. web=2)",
                pair
            );
        }
        let process_type = parts[0].to_string();
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

        let _: OkResponse = ctx
            .api_client
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
