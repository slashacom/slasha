use anyhow::Result;
use colored::Colorize;
use serde::{Deserialize, Serialize};
use serde_json::json;
use slasha_db::deployment::{Deployment, DeploymentStatus};

use crate::{
    clap_app::{DeploymentsCommand, LogArgs},
    commands::{logs::display_logs, resolve::resolve_deployment_id, responses::OkResponse},
    context::Context,
    http::ApiClient,
    output::{cli_info, cli_label, cli_success, confirm_action, print_table, spinner},
};

#[derive(Deserialize, Serialize)]
pub struct DeploymentItemResponse {
    pub deployment: Deployment,
}

#[derive(Deserialize, Serialize)]
pub struct DeploymentListResponse {
    pub deployments: Vec<Deployment>,
}

pub async fn dispatch(
    cmd: DeploymentsCommand,
    server_override: Option<&str>,
    app_override: Option<&str>,
) -> Result<()> {
    let ctx = Context::new(server_override, app_override)?;
    let (client, slug) = ctx.require_context()?;

    match cmd {
        DeploymentsCommand::List => handle_list(client, slug).await,
        DeploymentsCommand::Stop { deployment_id } => {
            handle_stop(client, slug, deployment_id).await
        }
        DeploymentsCommand::Restart { deployment_id } => {
            handle_restart(client, slug, deployment_id).await
        }
        DeploymentsCommand::Redeploy { deployment_id } => {
            handle_redeploy(client, slug, deployment_id).await
        }
        DeploymentsCommand::Rollback { deployment_id } => {
            handle_rollback(client, slug, deployment_id).await
        }
        DeploymentsCommand::Delete { deployment_id, yes } => {
            handle_delete(client, slug, deployment_id, yes).await
        }
    }
}

pub async fn handle_trigger(
    commit: Option<String>,
    follow: bool,
    server_override: Option<&str>,
    app_override: Option<&str>,
) -> Result<()> {
    let ctx = Context::new(server_override, app_override)?;
    let (client, app_slug) = ctx.require_context()?;

    let payload = match &commit {
        Some(sha) => json!({ "commit_sha": sha }),
        None => json!({ "commit_sha": null }),
    };

    let res: DeploymentItemResponse = {
        let _spin = spinner("Triggering deployment...");
        client
            .post(&format!("/api/apps/{}/deployments", app_slug), &payload)
            .await?
    };

    cli_success("Deployment triggered.");
    cli_label("Commit", &res.deployment.commit_sha);

    if follow {
        display_logs(
            client,
            &format!("/api/apps/{}/deployments/{}", app_slug, res.deployment.id),
            app_slug,
            &LogArgs {
                follow: true,
                limit: 0,
                search: None,
                prefix: None,
                stream: None,
            },
        )
        .await?;
    } else {
        cli_info("\nFollow logs: slasha logs --follow");
    }

    Ok(())
}

async fn handle_list(client: &ApiClient, slug: &str) -> Result<()> {
    let res: DeploymentListResponse = client
        .get(&format!("/api/apps/{}/deployments", slug))
        .await?;

    if res.deployments.is_empty() {
        cli_info(format!("No deployments found for app '{}'.", slug));
    } else {
        let mut rows: Vec<Vec<String>> = res
            .deployments
            .iter()
            .map(|d| {
                vec![
                    d.id.to_string(),
                    d.commit_sha.to_string(),
                    format_status(d.status),
                    d.created_at.format("%Y-%m-%d %H:%M").to_string(),
                ]
            })
            .collect();
        rows.sort_by(|a, b| b[3].cmp(&a[3]));
        print_table(&["ID", "COMMIT", "STATUS", "CREATED"], rows);
    }

    Ok(())
}

pub async fn handle_logs(
    deployment_id_arg: Option<String>,
    args: LogArgs,
    server_override: Option<&str>,
    app_override: Option<&str>,
) -> Result<()> {
    let ctx = Context::new(server_override, app_override)?;
    let (client, slug) = ctx.require_context()?;

    let deployment_id = resolve_deployment_id(client, slug, deployment_id_arg).await?;

    display_logs(
        client,
        &format!("/api/apps/{}/deployments/{}", slug, deployment_id),
        slug,
        &args,
    )
    .await
}

async fn handle_stop(
    client: &ApiClient,
    slug: &str,
    deployment_id_arg: Option<String>,
) -> Result<()> {
    let deployment_id = resolve_deployment_id(client, slug, deployment_id_arg).await?;

    let _spin = spinner("Stopping deployment...");
    let _: OkResponse = client
        .post(
            &format!("/api/apps/{}/deployments/{}/stop", slug, deployment_id),
            &json!({}),
        )
        .await?;

    cli_success(format!("Deployment {} stop triggered.", deployment_id));

    Ok(())
}

async fn handle_restart(
    client: &ApiClient,
    slug: &str,
    deployment_id_arg: Option<String>,
) -> Result<()> {
    let deployment_id = resolve_deployment_id(client, slug, deployment_id_arg).await?;

    let _spin = spinner("Restarting deployment...");
    let _: OkResponse = client
        .post(
            &format!("/api/apps/{}/deployments/{}/restart", slug, deployment_id),
            &json!({}),
        )
        .await?;

    cli_success(format!("Deployment {} restart triggered.", deployment_id));

    Ok(())
}

async fn handle_redeploy(
    client: &ApiClient,
    slug: &str,
    deployment_id_arg: Option<String>,
) -> Result<()> {
    let deployment_id = resolve_deployment_id(client, slug, deployment_id_arg).await?;

    let _spin = spinner("Redeploying...");
    let res: DeploymentItemResponse = client
        .post(
            &format!("/api/apps/{}/deployments/{}/redeploy", slug, deployment_id),
            &json!({}),
        )
        .await?;

    cli_success(format!(
        "Redeploy triggered for deployment {}.",
        res.deployment.id
    ));
    cli_info("\nFollow logs: slasha logs --follow");

    Ok(())
}

async fn handle_rollback(
    client: &ApiClient,
    slug: &str,
    deployment_id_arg: Option<String>,
) -> Result<()> {
    let res: DeploymentListResponse = client
        .get(&format!("/api/apps/{}/deployments", slug))
        .await?;

    let deployment_id = if let Some(id) = deployment_id_arg {
        if res
            .deployments
            .iter()
            .any(|d| d.id == id && d.status == DeploymentStatus::Running)
        {
            anyhow::bail!("Deployment {} is already running", id);
        }
        id
    } else {
        res.deployments
            .into_iter()
            .filter(|d| d.status == DeploymentStatus::Stopped)
            .max_by_key(|d| d.created_at)
            .map(|d| d.id)
            .ok_or_else(|| anyhow::anyhow!("No previous deployment available for rollback"))?
    };

    let _spin = spinner("Rolling back deployment...");
    let res: DeploymentItemResponse = client
        .post(
            &format!("/api/apps/{}/deployments/{}/rollback", slug, deployment_id),
            &json!({}),
        )
        .await?;

    cli_success(format!(
        "Rollback triggered to deployment {}.",
        res.deployment.id
    ));
    cli_label("Commit", &res.deployment.commit_sha);

    Ok(())
}

async fn handle_delete(
    client: &ApiClient,
    slug: &str,
    deployment_id_arg: Option<String>,
    yes: bool,
) -> Result<()> {
    let deployment_id = resolve_deployment_id(client, slug, deployment_id_arg).await?;

    if !confirm_action(yes, &format!("Delete deployment {}?", deployment_id.red()))? {
        return Ok(());
    }

    let _spin = spinner("Deleting deployment...");
    let _: OkResponse = client
        .delete(&format!("/api/apps/{}/deployments/{}", slug, deployment_id))
        .await?;

    cli_success(format!("Deployment {} deleted.", deployment_id));

    Ok(())
}

/// Formats a deployment status enum into an ANSI colored string.
fn format_status(status: DeploymentStatus) -> String {
    match status {
        DeploymentStatus::Running => status.to_string().green().to_string(),
        DeploymentStatus::Building | DeploymentStatus::Pending => {
            status.to_string().yellow().to_string()
        }
        DeploymentStatus::Failed => status.to_string().red().to_string(),
        DeploymentStatus::Stopped => status.to_string().dimmed().to_string(),
    }
}
