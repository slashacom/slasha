use std::collections::HashMap;

use anyhow::{Context, Result};
use serde_json::json;

use crate::{
    clap_app::ServiceEnvCommand,
    output::{cli_error, cli_info, cli_success, output, print_table},
    state::AppState,
};

pub async fn dispatch(
    state: &AppState,
    slug: &str,
    service_id: &str,
    cmd: ServiceEnvCommand,
) -> Result<()> {
    match cmd {
        ServiceEnvCommand::List => handle_list(state, slug, service_id).await,
        ServiceEnvCommand::Set { pairs } => handle_set(state, slug, service_id, &pairs).await,
        ServiceEnvCommand::Unset { keys } => handle_unset(state, slug, service_id, &keys).await,
    }
}

pub async fn handle_list(state: &AppState, slug: &str, service_id: &str) -> Result<()> {
    let env_data = state
        .api_client
        .get(&format!("/api/apps/{}/services/{}/env", slug, service_id))
        .await?;

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

pub async fn handle_set(
    state: &AppState,
    slug: &str,
    service_id: &str,
    pairs: &[String],
) -> Result<()> {
    let mut current = fetch_vars(state, slug, service_id).await?;

    for pair in pairs {
        let (k, v) = pair
            .split_once('=')
            .with_context(|| format!("'{}' is not KEY=VALUE", pair))?;
        current.insert(k.to_string(), v.to_string());
    }

    let update_res = state
        .api_client
        .put(
            &format!("/api/apps/{}/services/{}/env", slug, service_id),
            &json!({ "vars": current }),
        )
        .await?;

    output(state.output_mode, &update_res["env_vars"], || {
        cli_success("Service env vars updated.");
    })?;

    Ok(())
}

pub async fn handle_unset(
    state: &AppState,
    slug: &str,
    service_id: &str,
    keys: &[String],
) -> Result<()> {
    let mut current = fetch_vars(state, slug, service_id).await?;

    for key in keys {
        if current.remove(key).is_none() {
            cli_error(format!("Key '{}' not found — skipping.", key));
        }
    }

    let update_res = state
        .api_client
        .put(
            &format!("/api/apps/{}/services/{}/env", slug, service_id),
            &json!({ "vars": current }),
        )
        .await?;

    output(state.output_mode, &update_res["env_vars"], || {
        cli_success("Service env vars updated.");
    })?;

    Ok(())
}

async fn fetch_vars(
    state: &AppState,
    slug: &str,
    service_id: &str,
) -> Result<HashMap<String, String>> {
    let env_data = state
        .api_client
        .get(&format!("/api/apps/{}/services/{}/env", slug, service_id))
        .await?;

    serde_json::from_value(env_data["env_vars"].clone()).context("Failed to parse env vars")
}
