use anyhow::{Context, Result};
use serde_json::json;

use crate::{
    deployments::resolve_deployment_id,
    output::{cli_success, output, spinner},
    state::AppState,
};

pub async fn handle_scale(state: &AppState, slug: &str, pairs: Vec<String>) -> Result<()> {
    let deployment_id = resolve_deployment_id(state, slug, None).await?;

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
        let pb = if !state.output_mode.is_json() {
            Some(spinner(&format!(
                "Scaling {} to {}...",
                process_type, count
            )))
        } else {
            None
        };

        let res = state
            .api_client
            .post(
                &format!("/api/apps/{}/deployments/{}/scale", slug, deployment_id),
                &json!({
                    "process_type": process_type,
                    "count": count
                }),
            )
            .await;

        if let Some(pb) = pb {
            pb.finish_and_clear();
        }

        let payload = res?;

        output(state.output_mode, &payload, || {
            cli_success(format!("Scaled {} to {}.", process_type, count));
        })?;
    }

    Ok(())
}
