use std::collections::HashMap;

use anyhow::{Context as _, Result};
use serde_json::json;

use crate::{
    app_env::EnvVarsResponse,
    clap_app::ServiceEnvCommand,
    context::Context,
    output::{cli_error, cli_info, cli_success, print_table, spinner},
    resolve::resolve_service_id,
};

pub async fn dispatch(
    ctx: &Context,
    slug: &str,
    service_name_or_id: &str,
    cmd: ServiceEnvCommand,
) -> Result<()> {
    let service_id = resolve_service_id(&ctx.api_client, slug, service_name_or_id).await?;
    match cmd {
        ServiceEnvCommand::List => handle_list(ctx, slug, &service_id).await,
        ServiceEnvCommand::Set { pairs } => handle_set(ctx, slug, &service_id, &pairs).await,
        ServiceEnvCommand::Unset { keys } => handle_unset(ctx, slug, &service_id, &keys).await,
    }
}

pub async fn handle_list(ctx: &Context, slug: &str, service_id: &str) -> Result<()> {
    let res: EnvVarsResponse = ctx
        .api_client
        .get(&format!("/api/apps/{}/services/{}/env", slug, service_id))
        .await?;

    if res.env_vars.is_empty() {
        cli_info("No env vars set.");
    } else {
        let mut rows: Vec<Vec<String>> =
            res.env_vars.into_iter().map(|(k, v)| vec![k, v]).collect();
        rows.sort_by(|a, b| a[0].cmp(&b[0]));
        print_table(&["KEY", "VALUE"], rows);
    }

    Ok(())
}

pub async fn handle_set(
    ctx: &Context,
    slug: &str,
    service_id: &str,
    pairs: &[String],
) -> Result<()> {
    let mut current = fetch_vars(ctx, slug, service_id).await?;

    for pair in pairs {
        let (k, v) = pair
            .split_once('=')
            .with_context(|| format!("'{}' is not KEY=VALUE", pair))?;
        current.insert(k.to_string(), v.to_string());
    }

    let _spin = spinner("Updating service environment variables...");
    let _: EnvVarsResponse = ctx
        .api_client
        .put(
            &format!("/api/apps/{}/services/{}/env", slug, service_id),
            &json!({ "vars": current }),
        )
        .await?;

    cli_success("Service env vars updated.");

    Ok(())
}

pub async fn handle_unset(
    ctx: &Context,
    slug: &str,
    service_id: &str,
    keys: &[String],
) -> Result<()> {
    let mut current = fetch_vars(ctx, slug, service_id).await?;

    for key in keys {
        if current.remove(key).is_none() {
            cli_error(format!("Key '{}' not found — skipping.", key));
        }
    }

    let _spin = spinner("Updating service environment variables...");
    let _: EnvVarsResponse = ctx
        .api_client
        .put(
            &format!("/api/apps/{}/services/{}/env", slug, service_id),
            &json!({ "vars": current }),
        )
        .await?;

    cli_success("Service env vars updated.");

    Ok(())
}

/// Fetches environment variables configured for an attached service.
///
/// # Arguments
///
/// * `ctx` - Execution context ([`Context`]).
/// * `slug` - Target application slug.
/// * `service_id` - Target service ID.
///
/// # Returns
///
/// A key-value map of service environment variables.
async fn fetch_vars(
    ctx: &Context,
    slug: &str,
    service_id: &str,
) -> Result<HashMap<String, String>> {
    let res: EnvVarsResponse = ctx
        .api_client
        .get(&format!("/api/apps/{}/services/{}/env", slug, service_id))
        .await?;

    Ok(res.env_vars)
}
