use std::collections::HashMap;

use anyhow::{Context as _, Result};
use serde::{Deserialize, Serialize};
use serde_json::json;

use crate::{
    clap_app::AppEnvCommand,
    context::Context,
    output::{cli_error, cli_info, cli_success, print_table, spinner},
    resolve::resolve_slug,
};

pub async fn dispatch(ctx: &Context, slug_arg: Option<String>, cmd: AppEnvCommand) -> Result<()> {
    let slug = resolve_slug(slug_arg)?;
    match cmd {
        AppEnvCommand::List => handle_list(ctx, &slug).await,
        AppEnvCommand::Set { pairs } => handle_set(ctx, &slug, &pairs).await,
        AppEnvCommand::Unset { keys } => handle_unset(ctx, &slug, &keys).await,
    }
}

#[derive(Deserialize, Serialize)]
pub struct EnvVarsResponse {
    pub env_vars: HashMap<String, String>,
}

pub async fn handle_list(ctx: &Context, slug: &str) -> Result<()> {
    let res: EnvVarsResponse = ctx
        .api_client
        .get(&format!("/api/apps/{}/env", slug))
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

pub async fn handle_set(ctx: &Context, slug: &str, pairs: &[String]) -> Result<()> {
    let mut current = fetch_vars(ctx, slug).await?;

    for pair in pairs {
        let (k, v) = pair
            .split_once('=')
            .with_context(|| format!("'{}' is not KEY=VALUE", pair))?;
        current.insert(k.to_string(), v.to_string());
    }

    let _spin = spinner("Updating environment variables...");
    let _: EnvVarsResponse = ctx
        .api_client
        .put(
            &format!("/api/apps/{}/env", slug),
            &json!({ "vars": current }),
        )
        .await?;

    cli_success(format!("Env vars updated for app '{}'.", slug));

    Ok(())
}

pub async fn handle_unset(ctx: &Context, slug: &str, keys: &[String]) -> Result<()> {
    let mut current = fetch_vars(ctx, slug).await?;

    for key in keys {
        if current.remove(key).is_none() {
            cli_error(format!("Key '{}' not found — skipping.", key));
        }
    }

    let _spin = spinner("Updating environment variables...");
    let _: EnvVarsResponse = ctx
        .api_client
        .put(
            &format!("/api/apps/{}/env", slug),
            &json!({ "vars": current }),
        )
        .await?;

    cli_success(format!("Env vars updated for app '{}'.", slug));

    Ok(())
}

/// Fetches all environment variables configured for an application slug.
///
/// # Arguments
///
/// * `ctx` - Execution context ([`Context`]).
/// * `slug` - Target application slug.
///
/// # Returns
///
/// A key-value map of environment variables.
async fn fetch_vars(ctx: &Context, slug: &str) -> Result<HashMap<String, String>> {
    let res: EnvVarsResponse = ctx
        .api_client
        .get(&format!("/api/apps/{}/env", slug))
        .await?;

    Ok(res.env_vars)
}
