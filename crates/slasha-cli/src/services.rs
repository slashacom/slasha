use anyhow::{Context as _, Result};
use colored::Colorize;
use futures_util::StreamExt;
use serde::{Deserialize, Serialize};
use serde_json::json;
use slasha_db::service::{Service, ServiceKind, ServiceStatus};
use tokio::{
    fs::File,
    io::{AsyncWriteExt, stdout},
};

use crate::{
    clap_app::ServicesCommand,
    context::Context,
    output::{cli_info, cli_label, cli_success, confirm_action, print_table, spinner, stream_logs},
    resolve::{resolve_service_id, resolve_slug},
    service_env,
};

/// Formats a service status enum into an ANSI colored string.
///
/// # Arguments
///
/// * `status` - Service status ([`ServiceStatus`]).
///
/// # Returns
///
/// Colored status string representation.
fn format_status(status: ServiceStatus) -> String {
    match status {
        ServiceStatus::Running => status.to_string().green().to_string(),
        ServiceStatus::Provisioning => status.to_string().yellow().to_string(),
        ServiceStatus::Failed => status.to_string().red().to_string(),
        ServiceStatus::Stopped => status.to_string().dimmed().to_string(),
    }
}

pub async fn dispatch(ctx: &Context, slug_arg: Option<String>, cmd: ServicesCommand) -> Result<()> {
    let slug = resolve_slug(slug_arg)?;
    match cmd {
        ServicesCommand::List => handle_list(ctx, &slug).await,
        ServicesCommand::Provision {
            kind,
            name,
            version,
        } => handle_create(ctx, &slug, &kind, &name, &version).await,
        ServicesCommand::Restart { service } => handle_restart(ctx, &slug, &service).await,
        ServicesCommand::Redeploy { service } => handle_redeploy(ctx, &slug, &service).await,
        ServicesCommand::Stop { service, yes } => handle_stop(ctx, &slug, &service, yes).await,
        ServicesCommand::Delete { service, yes } => handle_delete(ctx, &slug, &service, yes).await,
        ServicesCommand::Logs { service, follow } => {
            handle_logs(ctx, &slug, &service, follow).await
        }
        ServicesCommand::Env { service, command } => {
            service_env::dispatch(ctx, &slug, &service, command).await
        }
        ServicesCommand::Backup { service, file } => {
            handle_backup(ctx, &slug, &service, file).await
        }
        ServicesCommand::Proxy {
            service,
            port,
            no_secret,
        } => crate::proxy::handle_proxy(ctx, &slug, &service, port, no_secret).await,
    }
}

#[derive(Deserialize, Serialize)]
pub struct ServiceListResponse {
    pub services: Vec<Service>,
}

pub async fn handle_list(ctx: &Context, slug: &str) -> Result<()> {
    let res: ServiceListResponse = ctx
        .api_client
        .get(&format!("/api/apps/{}/services", slug))
        .await?;

    if res.services.is_empty() {
        cli_info("No services attached. Run `slasha services provision` to add one.");
    } else {
        print_table(
            &["ID", "NAME", "KIND", "VERSION", "STATUS"],
            res.services
                .iter()
                .map(|s| {
                    vec![
                        s.id.to_string(),
                        s.name.clone(),
                        s.kind.to_string(),
                        s.version.clone(),
                        format_status(s.status),
                    ]
                })
                .collect(),
        );
    }

    Ok(())
}

#[derive(Deserialize, Serialize)]
pub struct ServiceItemResponse {
    pub service: Service,
}

pub async fn handle_create(
    ctx: &Context,
    slug: &str,
    kind: &ServiceKind,
    name: &str,
    version: &str,
) -> Result<()> {
    let default_env = fetch_default_env(ctx, kind).await?;

    let _spin = spinner("Provisioning service...");
    let res: ServiceItemResponse = ctx
        .api_client
        .post(
            &format!("/api/apps/{}/services", slug),
            &json!({
                "kind": kind,
                "name": name,
                "version": version,
                "env_vars": default_env,
            }),
        )
        .await?;

    cli_success("Service provisioning started.");
    cli_label("ID", &res.service.id);
    cli_label("Name", &res.service.name);
    cli_label("Kind", res.service.kind);
    cli_label("Version", &res.service.version);
    cli_info(format!(
        "\nFollow service logs: slasha services logs {} --follow",
        res.service.name
    ));
    cli_info(format!(
        "Connect locally:    slasha services proxy {} {}",
        slug, res.service.name
    ));

    Ok(())
}

#[derive(Deserialize, Serialize)]
pub struct OkResponse {
    pub ok: bool,
}

pub async fn handle_restart(ctx: &Context, slug: &str, service: &str) -> Result<()> {
    let service_id = resolve_service_id(&ctx.api_client, slug, service).await?;

    let _spin = spinner("Restarting service...");
    let _: OkResponse = ctx
        .api_client
        .post(
            &format!("/api/apps/{}/services/{}/restart", slug, service_id),
            &json!({}),
        )
        .await?;

    cli_success(format!("Service {} restart triggered.", service));

    Ok(())
}

pub async fn handle_redeploy(ctx: &Context, slug: &str, service: &str) -> Result<()> {
    let service_id = resolve_service_id(&ctx.api_client, slug, service).await?;

    let _spin = spinner("Redeploying service...");
    let _: OkResponse = ctx
        .api_client
        .post(
            &format!("/api/apps/{}/services/{}/redeploy", slug, service_id),
            &json!({}),
        )
        .await?;

    cli_success(format!("Service {} redeploy triggered.", service));

    Ok(())
}

pub async fn handle_stop(ctx: &Context, slug: &str, service: &str, yes: bool) -> Result<()> {
    let service_id = resolve_service_id(&ctx.api_client, slug, service).await?;

    if !confirm_action(yes, &format!("Stop service {}?", service.red()))? {
        return Ok(());
    }

    let _spin = spinner("Stopping service...");
    let _: OkResponse = ctx
        .api_client
        .post(
            &format!("/api/apps/{}/services/{}/stop", slug, service_id),
            &json!({}),
        )
        .await?;

    cli_success(format!("Service {} stop triggered.", service));

    Ok(())
}

pub async fn handle_delete(ctx: &Context, slug: &str, service: &str, yes: bool) -> Result<()> {
    let service_id = resolve_service_id(&ctx.api_client, slug, service).await?;

    if !confirm_action(yes, &format!("Delete service {}?", service.red()))? {
        return Ok(());
    }

    let _spin = spinner("Deleting service...");
    let _: OkResponse = ctx
        .api_client
        .delete(&format!("/api/apps/{}/services/{}", slug, service_id))
        .await?;

    cli_success(format!("Service {} deleted.", service));

    Ok(())
}

pub async fn handle_logs(ctx: &Context, slug: &str, service: &str, follow: bool) -> Result<()> {
    let service_id = resolve_service_id(&ctx.api_client, slug, service).await?;

    if follow {
        let res = ctx
            .api_client
            .get_stream(&format!(
                "/api/apps/{}/services/{}/stream",
                slug, service_id
            ))
            .await?;

        stream_logs(res).await?;
    } else {
        let data: crate::deployments::DeploymentLogsResponse = ctx
            .api_client
            .get(&format!(
                "/api/apps/{}/services/{}/logs?limit=2000",
                slug, service_id
            ))
            .await?;

        for rec in data.logs {
            let timestamp = rec["timestamp"].as_str().unwrap_or("").dimmed();
            let prefix = rec["prefix"]
                .as_str()
                .map(|p| format!("[{}]", p).cyan())
                .unwrap_or_default();
            let msg = rec["message"].as_str().unwrap_or("");

            if prefix.is_empty() {
                cli_info(format!("{} {}", timestamp, msg));
            } else {
                cli_info(format!("{} {} {}", timestamp, prefix, msg));
            }
        }
    }

    Ok(())
}

#[derive(Deserialize, Serialize)]
pub struct ServiceKindsResponse {
    pub kinds: Vec<serde_json::Value>,
}

/// Fetches default environment variables associated with a service kind.
///
/// # Arguments
///
/// * `ctx` - Execution context ([`Context`]).
/// * `kind` - Service kind ([`ServiceKind`]).
///
/// # Returns
///
/// A key-value map of default environment variables for the specified service kind.
async fn fetch_default_env(
    ctx: &Context,
    kind: &ServiceKind,
) -> Result<std::collections::HashMap<String, String>> {
    let res: ServiceKindsResponse = ctx
        .api_client
        .get("/api/services/kinds")
        .await
        .context("Failed to fetch supported service kinds")?;

    let kind_str = kind.to_string();
    for k in res.kinds {
        if k["name"].as_str().unwrap_or("") == kind_str {
            return serde_json::from_value(k["default_env_vars"].clone())
                .context("Failed to parse default env vars for service kind");
        }
    }

    Ok(Default::default())
}

pub async fn handle_backup(
    ctx: &Context,
    slug: &str,
    service: &str,
    file_path: Option<String>,
) -> Result<()> {
    let service_id = resolve_service_id(&ctx.api_client, slug, service).await?;

    let res = ctx
        .api_client
        .get_stream(&format!(
            "/api/apps/{}/services/{}/backup",
            slug, service_id
        ))
        .await?;

    let mut stream = res.bytes_stream();

    match file_path {
        Some(path) => {
            let mut file = File::create(&path)
                .await
                .with_context(|| format!("Failed to create file: {}", path))?;

            cli_info(format!("Writing backup to {}…", path));
            let mut total: u64 = 0;

            while let Some(chunk) = stream.next().await {
                let chunk = chunk.context("Stream error")?;
                total += chunk.len() as u64;
                file.write_all(&chunk).await.context("Write error")?;
            }

            file.flush().await.context("Flush error")?;
            cli_success(format!("Done. {} bytes written.", total));
        }
        None => {
            let mut out = stdout();
            while let Some(chunk) = stream.next().await {
                let chunk = chunk.context("Stream error")?;
                out.write_all(&chunk).await.context("Write error")?;
            }
            out.flush().await.context("Flush error")?;
        }
    }

    Ok(())
}
