use std::collections::HashMap;

use anyhow::{Context as _, Result};
use serde_json::json;

use crate::{
    clap_app::AppEnvCommand,
    commands::responses::EnvVarsResponse,
    context::Context,
    http::ApiClient,
    output::{cli_error, cli_info, cli_success, print_table, spinner},
};

pub async fn dispatch(
    cmd: AppEnvCommand,
    server_override: Option<&str>,
    app_override: Option<&str>,
) -> Result<()> {
    let ctx = Context::new(server_override, app_override)?;
    let (client, slug) = ctx.require_context()?;

    match cmd {
        AppEnvCommand::List => handle_list(client, slug).await,
        AppEnvCommand::Set { pairs } => handle_set(client, slug, &pairs).await,
        AppEnvCommand::Unset { keys } => handle_unset(client, slug, &keys).await,
    }
}

async fn handle_list(client: &ApiClient, slug: &str) -> Result<()> {
    let res: EnvVarsResponse = client.get(&format!("/api/apps/{}/env", slug)).await?;

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

async fn handle_set(client: &ApiClient, slug: &str, pairs: &[String]) -> Result<()> {
    let mut current = fetch_vars(client, slug).await?;

    for pair in pairs {
        let (k, v) = pair
            .split_once('=')
            .with_context(|| format!("'{}' is not KEY=VALUE", pair))?;
        current.insert(k.to_string(), v.to_string());
    }

    let _spin = spinner("Updating environment variables...");
    let _: EnvVarsResponse = client
        .put(
            &format!("/api/apps/{}/env", slug),
            &json!({ "vars": current }),
        )
        .await?;

    cli_success(format!("Env vars updated for app '{}'.", slug));

    Ok(())
}

async fn handle_unset(client: &ApiClient, slug: &str, keys: &[String]) -> Result<()> {
    let mut current = fetch_vars(client, slug).await?;

    for key in keys {
        if current.remove(key).is_none() {
            cli_error(format!("Key '{}' not found — skipping.", key));
        }
    }

    let _spin = spinner("Updating environment variables...");
    let _: EnvVarsResponse = client
        .put(
            &format!("/api/apps/{}/env", slug),
            &json!({ "vars": current }),
        )
        .await?;

    cli_success(format!("Env vars updated for app '{}'.", slug));

    Ok(())
}

/// Fetches all environment variables configured for an application slug.
async fn fetch_vars(client: &ApiClient, slug: &str) -> Result<HashMap<String, String>> {
    let res: EnvVarsResponse = client.get(&format!("/api/apps/{}/env", slug)).await?;
    Ok(res.env_vars)
}
