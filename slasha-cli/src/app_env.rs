use std::collections::HashMap;

use anyhow::{Context, Result};
use serde_json::json;

use crate::{
    clap_app::AppEnvCommand,
    output::{cli_error, cli_info, cli_success, output, print_table},
    state::AppState,
};

pub async fn dispatch(state: &AppState, slug: &str, cmd: AppEnvCommand) -> Result<()> {
    match cmd {
        AppEnvCommand::List => handle_list(state, slug).await,
        AppEnvCommand::Set { pairs } => handle_set(state, slug, &pairs).await,
        AppEnvCommand::Unset { keys } => handle_unset(state, slug, &keys).await,
    }
}

pub async fn handle_list(state: &AppState, slug: &str) -> Result<()> {
    let env_data = state.client.get(&format!("/api/apps/{}/env", slug)).await?;

    let vars: HashMap<String, String> =
        serde_json::from_value(env_data["env_vars"].clone()).context("Failed to parse env vars")?;

    output(state.output_mode, &vars, || {
        if vars.is_empty() {
            cli_info("No env vars set.");
        } else {
            let mut rows: Vec<Vec<String>> = vars
                .iter()
                .map(|(k, v)| vec![k.clone(), v.clone()])
                .collect();
            rows.sort_by(|a, b| a[0].cmp(&b[0]));
            print_table(&["KEY", "VALUE"], rows);
        }
    })?;

    Ok(())
}

pub async fn handle_set(state: &AppState, slug: &str, pairs: &[String]) -> Result<()> {
    let mut current = fetch_vars(state, slug).await?;

    for pair in pairs {
        let (k, v) = pair
            .split_once('=')
            .with_context(|| format!("'{}' is not KEY=VALUE", pair))?;
        current.insert(k.to_string(), v.to_string());
    }

    let update_res = state
        .client
        .put(
            &format!("/api/apps/{}/env", slug),
            &json!({ "vars": current }),
        )
        .await?;

    output(state.output_mode, &update_res["env_vars"], || {
        cli_success(format!("Env vars updated for {}.", slug));
    })?;

    Ok(())
}

pub async fn handle_unset(state: &AppState, slug: &str, keys: &[String]) -> Result<()> {
    let mut current = fetch_vars(state, slug).await?;

    for key in keys {
        if current.remove(key).is_none() {
            cli_error(format!("Key '{}' not found — skipping.", key));
        }
    }

    let update_res = state
        .client
        .put(
            &format!("/api/apps/{}/env", slug),
            &json!({ "vars": current }),
        )
        .await?;

    output(state.output_mode, &update_res["env_vars"], || {
        cli_success(format!("Env vars updated for {}.", slug));
    })?;

    Ok(())
}

async fn fetch_vars(state: &AppState, slug: &str) -> Result<HashMap<String, String>> {
    let env_data = state.client.get(&format!("/api/apps/{}/env", slug)).await?;

    serde_json::from_value(env_data["env_vars"].clone()).context("Failed to parse env vars")
}
