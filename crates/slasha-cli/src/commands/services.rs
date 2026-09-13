use anyhow::{Context as _, Result};
use colored::Colorize;
use serde::{Deserialize, Serialize};
use serde_json::json;
use slasha_db::service::{Service, ServiceKind};

use crate::{
    clap_app::{LogArgs, ServicesCommand},
    commands::{
        logs::display_logs, proxy, resolve::resolve_service_id, responses::OkResponse,
        service_backup, service_env,
    },
    context::Context,
    http::ApiClient,
    output::{cli_info, cli_label, cli_success, confirm_action, print_table, spinner},
};

#[derive(Deserialize, Serialize)]
pub struct ServiceListItem {
    pub service: Service,
    #[serde(default)]
    pub runtime_status: Option<String>,
}

#[derive(Deserialize, Serialize)]
pub struct ServiceListResponse {
    pub services: Vec<ServiceListItem>,
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
        ServicesCommand::Logs { service, args } => handle_logs(client, slug, &service, args).await,
        ServicesCommand::Env { service, command } => {
            service_env::dispatch(client, slug, &service, command).await
        }
        ServicesCommand::Backup { service, command } => {
            service_backup::dispatch(client, slug, &service, command).await
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
                .map(|item| {
                    let status = item
                        .runtime_status
                        .clone()
                        .unwrap_or_else(|| item.service.status.to_string());
                    vec![
                        item.service.name.clone(),
                        item.service.kind.to_string(),
                        item.service.version.clone(),
                        status,
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
    let name_trimmed = name.trim();
    let is_valid_name = !name_trimmed.is_empty()
        && name_trimmed.len() <= 63
        && name_trimmed
            .chars()
            .all(|c| c.is_ascii_lowercase() || c.is_ascii_digit() || c == '-')
        && !name_trimmed.starts_with('-')
        && !name_trimmed.ends_with('-');

    if !is_valid_name {
        anyhow::bail!(
            "Service name '{}' is invalid: must consist of lowercase alphanumeric characters or '-' and must start and end with an alphanumeric character (max 63 characters)",
            name
        );
    }

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

async fn handle_logs(client: &ApiClient, slug: &str, service: &str, args: LogArgs) -> Result<()> {
    let service_id = resolve_service_id(client, slug, service).await?;

    display_logs(
        client,
        &format!("/api/apps/{}/services/{}", slug, service_id),
        &format!("{}/{}", slug, service),
        &args,
    )
    .await
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

    for k in res.kinds {
        if k["name"].as_str().unwrap_or("") == kind.to_string() {
            return serde_json::from_value(k["default_env_vars"].clone())
                .context("Failed to parse default env vars for service kind");
        }
    }

    Ok(Default::default())
}
