use anyhow::{Context, Result};
use colored::Colorize;
use eventsource_stream::Eventsource;
use futures_util::StreamExt;
use serde_json::json;
use slasha_db::deployment::{Deployment, DeploymentStatus};

use crate::{
    clap_app::DeploymentsCommand,
    output::{
        cli_error, cli_info, cli_label, cli_success, confirm_action, output, print_table, spinner,
    },
    state::AppState,
};

pub async fn dispatch(state: &AppState, slug: &str, cmd: DeploymentsCommand) -> Result<()> {
    match cmd {
        DeploymentsCommand::List => handle_list(state, slug).await,
        DeploymentsCommand::Stop { deployment_id } => handle_stop(state, slug, deployment_id).await,
        DeploymentsCommand::Restart { deployment_id } => {
            handle_restart(state, slug, deployment_id).await
        }
        DeploymentsCommand::Redeploy { deployment_id } => {
            handle_redeploy(state, slug, deployment_id).await
        }
        DeploymentsCommand::Rollback { deployment_id } => {
            handle_rollback(state, slug, deployment_id).await
        }
        DeploymentsCommand::Delete { deployment_id, yes } => {
            handle_delete(state, slug, deployment_id, yes).await
        }
        DeploymentsCommand::Logs {
            deployment_id,
            follow,
        } => handle_logs(state, slug, deployment_id, follow).await,
    }
}

pub async fn handle_trigger(state: &AppState, slug: &str, commit: Option<String>) -> Result<()> {
    let payload = match &commit {
        Some(sha) => json!({ "commit_sha": sha }),
        None => json!({ "commit_sha": null }),
    };

    let pb = if !state.output_mode.is_json() {
        Some(spinner("Triggering deployment..."))
    } else {
        None
    };

    let payload = match state
        .api_client
        .post(&format!("/api/apps/{}/deployments", slug), &payload)
        .await
    {
        Ok(b) => {
            if let Some(pb) = &pb {
                pb.finish_and_clear();
            }
            b
        }
        Err(e) => {
            if let Some(pb) = &pb {
                pb.finish_and_clear();
            }
            return Err(e);
        }
    };

    let dep: Deployment = serde_json::from_value(payload["deployment"].clone())
        .context("Failed to parse deployment")?;

    output(state.output_mode, &dep, || {
        cli_success("Deployment triggered.");
        cli_label("ID", &dep.id);
        cli_label("Commit", &dep.commit_sha);
        cli_info(format!(
            "\nFollow logs: slasha logs --app {} --follow",
            slug
        ));
    })?;

    Ok(())
}

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

pub async fn handle_list(state: &AppState, slug: &str) -> Result<()> {
    let deployments_data = state
        .api_client
        .get(&format!("/api/apps/{}/deployments", slug))
        .await?;

    let deps: Vec<Deployment> = serde_json::from_value(deployments_data["deployments"].clone())
        .context("Failed to parse deployments")?;

    output(state.output_mode, &deps, || {
        if deps.is_empty() {
            cli_info(format!("No deployments found for app '{}'.", slug));
        } else {
            let mut rows: Vec<Vec<String>> = deps
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
    })?;

    Ok(())
}

pub async fn handle_logs(
    state: &AppState,
    slug: &str,
    deployment_id: Option<String>,
    follow: bool,
) -> Result<()> {
    let deployment_id = resolve_deployment_id(state, slug, deployment_id).await?;

    let res = state
        .api_client
        .get_stream(&format!(
            "/api/apps/{}/deployments/{}/logs",
            slug, deployment_id
        ))
        .await?;

    let mut stream = res.bytes_stream().eventsource();

    while let Some(event) = stream.next().await {
        match event {
            Ok(event) => {
                if event.data == "[done]" {
                    output(
                        state.output_mode,
                        &json!({ "type": "status", "event": "history_complete" }),
                        || {},
                    )?;

                    if !follow {
                        break;
                    }
                } else {
                    output(
                        state.output_mode,
                        &json!({ "type": "log", "message": event.data }),
                        || {
                            cli_info(&event.data);
                        },
                    )?;
                }
            }

            Err(e) => {
                cli_error(format!("Stream error: {}", e));
                break;
            }
        }
    }

    Ok(())
}

pub async fn handle_stop(
    state: &AppState,
    slug: &str,
    deployment_id: Option<String>,
) -> Result<()> {
    let deployment_id = resolve_deployment_id(state, slug, deployment_id).await?;

    state
        .api_client
        .post(
            &format!("/api/apps/{}/deployments/{}/stop", slug, deployment_id),
            &json!({}),
        )
        .await?;

    output(state.output_mode, &json!({ "ok": true }), || {
        cli_success(format!("Deployment {} stop triggered.", deployment_id));
    })?;

    Ok(())
}

pub async fn handle_restart(
    state: &AppState,
    slug: &str,
    deployment_id: Option<String>,
) -> Result<()> {
    let deployment_id = resolve_deployment_id(state, slug, deployment_id).await?;

    state
        .api_client
        .post(
            &format!("/api/apps/{}/deployments/{}/restart", slug, deployment_id),
            &json!({}),
        )
        .await?;

    output(state.output_mode, &json!({ "ok": true }), || {
        cli_success(format!("Deployment {} restart triggered.", deployment_id));
    })?;

    Ok(())
}

pub async fn handle_redeploy(
    state: &AppState,
    slug: &str,
    deployment_id: Option<String>,
) -> Result<()> {
    let deployment_id = resolve_deployment_id(state, slug, deployment_id).await?;

    let payload = state
        .api_client
        .post(
            &format!("/api/apps/{}/deployments/{}/redeploy", slug, deployment_id),
            &json!({}),
        )
        .await?;

    let dep: Deployment = serde_json::from_value(payload["deployment"].clone())
        .context("Failed to parse deployment")?;

    output(state.output_mode, &dep, || {
        cli_success(format!(
            "Redeploy triggered for deployment {}.",
            deployment_id
        ));
        cli_info(format!(
            "\nFollow logs: slasha logs --app {} --follow",
            slug
        ));
    })?;

    Ok(())
}

pub async fn handle_rollback(
    state: &AppState,
    slug: &str,
    deployment_id: Option<String>,
) -> Result<()> {
    let deployments_data = state
        .api_client
        .get(&format!("/api/apps/{}/deployments", slug))
        .await?;
    let deployments: Vec<Deployment> =
        serde_json::from_value(deployments_data["deployments"].clone())
            .context("Failed to parse deployments")?;

    let deployment_id = if let Some(id) = deployment_id {
        if deployments
            .iter()
            .any(|deployment| deployment.id == id && deployment.status == DeploymentStatus::Running)
        {
            anyhow::bail!("Deployment {} is already running", id);
        }
        id
    } else {
        deployments
            .into_iter()
            .filter(|deployment| deployment.status == DeploymentStatus::Stopped)
            .max_by_key(|deployment| deployment.created_at)
            .map(|deployment| deployment.id)
            .ok_or_else(|| anyhow::anyhow!("No previous deployment available for rollback"))?
    };

    let payload = state
        .api_client
        .post(
            &format!("/api/apps/{}/deployments/{}/rollback", slug, deployment_id),
            &json!({}),
        )
        .await?;

    let deployment: Deployment = serde_json::from_value(payload["deployment"].clone())
        .context("Failed to parse deployment")?;

    output(state.output_mode, &deployment, || {
        cli_success(format!(
            "Rollback triggered to deployment {}.",
            deployment_id
        ));
        cli_label("ID", &deployment.id);
        cli_label("Commit", &deployment.commit_sha);
    })?;

    Ok(())
}

pub async fn handle_delete(
    state: &AppState,
    slug: &str,
    deployment_id: Option<String>,
    yes: bool,
) -> Result<()> {
    let deployment_id = resolve_deployment_id(state, slug, deployment_id).await?;

    if !confirm_action(
        state.output_mode,
        yes,
        &format!("Delete deployment {}?", deployment_id.red()),
    )? {
        return Ok(());
    }

    state
        .api_client
        .delete(&format!("/api/apps/{}/deployments/{}", slug, deployment_id))
        .await?;

    output(state.output_mode, &json!({ "ok": true }), || {
        cli_success(format!("Deployment {} deleted.", deployment_id));
    })?;

    Ok(())
}

// return the latest deployment id if not provided else the provided id
pub async fn resolve_deployment_id(
    state: &AppState,
    slug: &str,
    deployment_id: Option<String>,
) -> Result<String> {
    if let Some(id) = deployment_id {
        return Ok(id);
    }

    let deployments_data = state
        .api_client
        .get(&format!("/api/apps/{}/deployments", slug))
        .await?;

    let deps: Vec<Deployment> = serde_json::from_value(deployments_data["deployments"].clone())
        .context("Failed to parse deployments")?;

    let target_dep = deps.into_iter().max_by_key(|d| d.created_at);

    match target_dep {
        Some(dep) => Ok(dep.id),
        None => {
            anyhow::bail!("No deployments found for app '{}'", slug);
        }
    }
}
