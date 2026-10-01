use std::collections::HashMap;

use anyhow::{Context as _, Result};
use serde_json::json;

use crate::{
    clap_app::AppEnvCommand,
    commands::{
        deployments::DeploymentItemResponse, resolve::resolve_running_deployment_id,
        responses::EnvVarsResponse,
    },
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
        AppEnvCommand::Set { pairs, deploy } => {
            handle_set(client, slug, &pairs).await?;
            after_change(client, slug, deploy).await
        }
        AppEnvCommand::Unset { keys, deploy } => {
            if !handle_unset(client, slug, &keys).await? {
                return Ok(());
            }
            after_change(client, slug, deploy).await
        }
        AppEnvCommand::Apply => apply(client, slug).await,
    }
}

/// Env vars are read when a deployment is created, so a change reaches
/// containers only through a new deployment; a restart keeps the old values.
async fn after_change(client: &ApiClient, slug: &str, deploy: bool) -> Result<()> {
    if deploy {
        return apply(client, slug).await;
    }

    cli_info(format!(
        "\nEnv vars are read when a deployment starts, so running containers keep the old values until the next deployment.\n\
         Apply them now with: slasha env --app {slug} apply"
    ));

    Ok(())
}

/// Releases the running deployment's image again with the current env vars.
///
/// This goes through the rollback endpoint, which creates a new deployment
/// from an existing one's commit and retained image (rebuilding only if the
/// image is gone), resolves the environment afresh, runs the release command
/// and switches traffic once the web process is ready.
async fn apply(client: &ApiClient, slug: &str) -> Result<()> {
    let running_id = resolve_running_deployment_id(client, slug).await.context(
        "Nothing to apply the environment to; it will be used by the next `slasha deploy`",
    )?;

    let res: DeploymentItemResponse = {
        let _spin = spinner("Applying environment...");
        client
            .post(
                &format!("/api/apps/{}/deployments/{}/rollback", slug, running_id),
                &json!({}),
            )
            .await?
    };

    cli_success(format!(
        "Deployment {} started from {} with the current environment.",
        res.deployment.id, running_id
    ));
    cli_info(format!(
        "\nFollow logs: slasha logs --app {} {} --follow",
        slug, res.deployment.id
    ));

    Ok(())
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

async fn handle_unset(client: &ApiClient, slug: &str, keys: &[String]) -> Result<bool> {
    let mut current = fetch_vars(client, slug).await?;
    let mut removed_count = 0;

    for key in keys {
        if current.remove(key).is_some() {
            removed_count += 1;
        } else {
            cli_error(format!("Key '{}' not found — skipping.", key));
        }
    }

    if removed_count == 0 {
        cli_info("No environment variables were modified.");
        return Ok(false);
    }

    let _spin = spinner("Updating environment variables...");
    let _: EnvVarsResponse = client
        .put(
            &format!("/api/apps/{}/env", slug),
            &json!({ "vars": current }),
        )
        .await?;

    cli_success(format!("Env vars updated for app '{}'.", slug));

    Ok(true)
}

/// Fetches all environment variables configured for an application slug.
async fn fetch_vars(client: &ApiClient, slug: &str) -> Result<HashMap<String, String>> {
    let res: EnvVarsResponse = client.get(&format!("/api/apps/{}/env", slug)).await?;
    Ok(res.env_vars)
}
