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
    commands::{
        proxy,
        resolve::resolve_service_id,
        responses::{LogsResponse, OkResponse},
        service_env,
    },
    context::Context,
    http::ApiClient,
    output::{cli_info, cli_label, cli_success, confirm_action, print_table, spinner, stream_logs},
};

#[derive(Deserialize, Serialize)]
pub struct ServiceListResponse {
    pub services: Vec<Service>,
}

#[derive(Deserialize, Serialize)]
pub struct ServiceItemResponse {
    pub service: Service,
}

#[derive(Deserialize, Serialize)]
pub struct ServiceKindsResponse {
    pub kinds: Vec<serde_json::Value>,
}

pub async fn dispatch(
    cmd: ServicesCommand,
    server_override: Option<&str>,
    app_override: Option<&str>,
) -> Result<()> {
    let ctx = Context::new(server_override, app_override)?;
    let (client, slug) = ctx.require_context()?;

    match cmd {
        ServicesCommand::List => handle_list(client, slug).await,
        ServicesCommand::Provision {
            kind,
            name,
            version,
        } => handle_create(client, slug, &kind, &name, version.as_deref()).await,
        ServicesCommand::Restart { service } => handle_restart(client, slug, &service).await,
        ServicesCommand::Redeploy { service } => handle_redeploy(client, slug, &service).await,
        ServicesCommand::Stop { service, yes } => handle_stop(client, slug, &service, yes).await,
        ServicesCommand::Delete { service, yes } => {
            handle_delete(client, slug, &service, yes).await
        }
        ServicesCommand::Logs { service, follow } => {
            handle_logs(client, slug, &service, follow).await
        }
        ServicesCommand::Env { service, command } => {
            service_env::dispatch(client, slug, &service, command).await
        }
        ServicesCommand::Backup { service, file } => {
            handle_backup(client, slug, &service, file).await
        }
        ServicesCommand::Proxy {
            service,
            port,
            no_secret,
        } => proxy::handle_proxy(client, slug, &service, port, no_secret).await,
    }
}

async fn handle_list(client: &ApiClient, slug: &str) -> Result<()> {
    let res: ServiceListResponse = client.get(&format!("/api/apps/{}/services", slug)).await?;

    if res.services.is_empty() {
        cli_info("No services attached. Run `slasha services provision` to add one.");
    } else {
        print_table(
            &["NAME", "KIND", "VERSION", "STATUS"],
            res.services
                .iter()
                .map(|s| {
                    vec![
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

async fn handle_create(
    client: &ApiClient,
    slug: &str,
    kind: &ServiceKind,
    name: &str,
    version: Option<&str>,
) -> Result<()> {
    let default_env = fetch_default_env(client, kind).await?;

    let resolved_version = match version {
        Some(v) if !v.trim().is_empty() => v,
        _ => kind
            .supported_versions()
            .first()
            .copied()
            .ok_or_else(|| anyhow::anyhow!("No supported versions for {:?}", kind))?,
    };

    let _spin = spinner("Provisioning service...");
    let res: ServiceItemResponse = client
        .post(
            &format!("/api/apps/{}/services", slug),
            &json!({
                "kind": kind,
                "name": name,
                "version": resolved_version,
                "env_vars": default_env,
            }),
        )
        .await?;

    cli_success("Service provisioning started.");
    cli_label("Name", &res.service.name);
    cli_label("Kind", res.service.kind);
    cli_label("Version", &res.service.version);
    cli_info(format!(
        "\nFollow service logs: slasha services logs {} --follow",
        res.service.name
    ));
    cli_info(format!(
        "Connect locally: slasha services proxy {}",
        res.service.name
    ));

    Ok(())
}

async fn handle_restart(client: &ApiClient, slug: &str, service: &str) -> Result<()> {
    let service_id = resolve_service_id(client, slug, service).await?;

    let _spin = spinner("Restarting service...");
    let _: OkResponse = client
        .post(
            &format!("/api/apps/{}/services/{}/restart", slug, service_id),
            &json!({}),
        )
        .await?;

    cli_success(format!("Service {} restart triggered.", service));

    Ok(())
}

async fn handle_redeploy(client: &ApiClient, slug: &str, service: &str) -> Result<()> {
    let service_id = resolve_service_id(client, slug, service).await?;

    let _spin = spinner("Redeploying service...");
    let _: OkResponse = client
        .post(
            &format!("/api/apps/{}/services/{}/redeploy", slug, service_id),
            &json!({}),
        )
        .await?;

    cli_success(format!("Service {} redeploy triggered.", service));

    Ok(())
}

async fn handle_stop(client: &ApiClient, slug: &str, service: &str, yes: bool) -> Result<()> {
    let service_id = resolve_service_id(client, slug, service).await?;

    if !confirm_action(yes, &format!("Stop service {}?", service.red()))? {
        return Ok(());
    }

    let _spin = spinner("Stopping service...");
    let _: OkResponse = client
        .post(
            &format!("/api/apps/{}/services/{}/stop", slug, service_id),
            &json!({}),
        )
        .await?;

    cli_success(format!("Service {} stop triggered.", service));

    Ok(())
}

async fn handle_delete(client: &ApiClient, slug: &str, service: &str, yes: bool) -> Result<()> {
    let service_id = resolve_service_id(client, slug, service).await?;

    if !confirm_action(yes, &format!("Delete service {}?", service.red()))? {
        return Ok(());
    }

    let _spin = spinner("Deleting service...");
    let _: OkResponse = client
        .delete(&format!("/api/apps/{}/services/{}", slug, service_id))
        .await?;

    cli_success(format!("Service {} deleted.", service));

    Ok(())
}

async fn handle_logs(client: &ApiClient, slug: &str, service: &str, follow: bool) -> Result<()> {
    let service_id = resolve_service_id(client, slug, service).await?;

    if follow {
        let res = client
            .get_stream(&format!(
                "/api/apps/{}/services/{}/stream",
                slug, service_id
            ))
            .await?;

        stream_logs(res).await?;
    } else {
        let data: LogsResponse = client
            .get(&format!(
                "/api/apps/{}/services/{}/logs?limit=2000",
                slug, service_id
            ))
            .await?;

        for rec in data.logs {
            let timestamp = rec
                .timestamp
                .format("%Y-%m-%d %H:%M:%S")
                .to_string()
                .dimmed();
            let prefix = rec
                .prefix
                .as_ref()
                .map(|p| format!("[{}]", p).cyan())
                .unwrap_or_default();

            if prefix.is_empty() {
                cli_info(format!("{} {}", timestamp, rec.message));
            } else {
                cli_info(format!("{} {} {}", timestamp, prefix, rec.message));
            }
        }
    }

    Ok(())
}

async fn handle_backup(
    client: &ApiClient,
    slug: &str,
    service: &str,
    file_path: Option<String>,
) -> Result<()> {
    let service_id = resolve_service_id(client, slug, service).await?;

    let res = client
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

/// Fetches default environment variables associated with a service kind.
///
/// # Arguments
///
/// * `client` - Reference to the API client ([`ApiClient`]).
/// * `kind` - Service kind ([`ServiceKind`]).
///
/// # Returns
///
/// A key-value map of default environment variables for the specified service kind.
async fn fetch_default_env(
    client: &ApiClient,
    kind: &ServiceKind,
) -> Result<std::collections::HashMap<String, String>> {
    let res: ServiceKindsResponse = client
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
