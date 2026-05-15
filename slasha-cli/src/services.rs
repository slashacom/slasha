use anyhow::{Context, Result};
use colored::Colorize;
use eventsource_stream::Eventsource;
use futures_util::StreamExt;
use serde_json::json;
use slasha_db::service::{Service, ServiceKind, ServiceStatus};

use crate::{
    clap_app::ServicesCommand,
    output::{cli_error, cli_info, cli_label, cli_success, confirm_action, output, print_table},
    service_env,
    state::AppState,
};

pub async fn dispatch(state: &AppState, slug: &str, cmd: ServicesCommand) -> Result<()> {
    match cmd {
        ServicesCommand::List => handle_list(state, slug).await,
        ServicesCommand::Restart { service } => handle_restart(state, slug, &service).await,
        ServicesCommand::Redeploy { service } => handle_redeploy(state, slug, &service).await,
        ServicesCommand::Stop { service, yes } => handle_stop(state, slug, &service, yes).await,
        ServicesCommand::Delete { service, yes } => handle_delete(state, slug, &service, yes).await,
        ServicesCommand::Logs { service, follow } => {
            handle_logs(state, slug, &service, follow).await
        }
        ServicesCommand::Env { service, command } => {
            service_env::dispatch(state, slug, &service, command).await
        }
    }
}

fn format_status(status: ServiceStatus) -> String {
    match status {
        ServiceStatus::Running => status.to_string().green().to_string(),
        ServiceStatus::Provisioning => status.to_string().yellow().to_string(),
        ServiceStatus::Failed => status.to_string().red().to_string(),
        ServiceStatus::Stopped => status.to_string().dimmed().to_string(),
    }
}

pub async fn handle_list(state: &AppState, slug: &str) -> Result<()> {
    let services_data = state
        .api_client
        .get(&format!("/api/apps/{}/services", slug))
        .await?;

    let svcs: Vec<Service> = serde_json::from_value(services_data["services"].clone())
        .context("Failed to parse services")?;

    output(state.output_mode, &svcs, || {
        if svcs.is_empty() {
            cli_info("No services attached. Run slasha provision to add one.");
        } else {
            print_table(
                &["ID", "NAME", "KIND", "VERSION", "STATUS"],
                svcs.iter()
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
    })?;

    Ok(())
}

pub async fn handle_create(
    state: &AppState,
    slug: &str,
    kind: &ServiceKind,
    name: &str,
    version: &str,
    expose: bool,
) -> Result<()> {
    let default_env = fetch_default_env(state, kind).await?;

    let provision_res = state
        .api_client
        .post(
            &format!("/api/apps/{}/services", slug),
            &json!({
                "kind": kind,
                "name": name,
                "version": version,
                "env_vars": default_env,
                "exposed": expose,
            }),
        )
        .await?;

    let svc: Service = serde_json::from_value(provision_res["service"].clone())
        .context("Failed to parse service")?;

    output(state.output_mode, &svc, || {
        cli_success("Service provisioning started.");
        cli_label("ID", &svc.id);
        cli_label("Name", &svc.name);
        cli_label("Kind", svc.kind);
        cli_label("Version", &svc.version);
        cli_label("Exposed", if expose { "Yes" } else { "No" });
        cli_info(format!(
            "\nFollow service logs: slasha services logs --app {} --service {} --follow",
            slug, svc.name
        ));
    })?;

    Ok(())
}

pub async fn handle_restart(state: &AppState, slug: &str, service: &str) -> Result<()> {
    let service_id = resolve_service_id(state, slug, service).await?;

    state
        .api_client
        .post(
            &format!("/api/apps/{}/services/{}/restart", slug, service_id),
            &json!({}),
        )
        .await?;

    output(state.output_mode, &json!({ "ok": true }), || {
        cli_success(format!("Service {} restart triggered.", service));
    })?;

    Ok(())
}

pub async fn handle_redeploy(state: &AppState, slug: &str, service: &str) -> Result<()> {
    let service_id = resolve_service_id(state, slug, service).await?;

    state
        .api_client
        .post(
            &format!("/api/apps/{}/services/{}/redeploy", slug, service_id),
            &json!({}),
        )
        .await?;

    output(state.output_mode, &json!({ "ok": true }), || {
        cli_success(format!("Service {} redeploy triggered.", service));
    })?;

    Ok(())
}

pub async fn handle_stop(state: &AppState, slug: &str, service: &str, yes: bool) -> Result<()> {
    let service_id = resolve_service_id(state, slug, service).await?;

    if !confirm_action(
        state.output_mode,
        yes,
        &format!("Stop service {}?", service.red()),
    )? {
        return Ok(());
    }

    state
        .api_client
        .post(
            &format!("/api/apps/{}/services/{}/stop", slug, service_id),
            &serde_json::Value::Null,
        )
        .await?;

    output(state.output_mode, &json!({ "ok": true }), || {
        cli_success(format!("Service {} stop triggered.", service));
    })?;

    Ok(())
}

pub async fn handle_delete(state: &AppState, slug: &str, service: &str, yes: bool) -> Result<()> {
    let service_id = resolve_service_id(state, slug, service).await?;

    if !confirm_action(
        state.output_mode,
        yes,
        &format!("Delete service {}?", service.red()),
    )? {
        return Ok(());
    }

    state
        .api_client
        .delete(&format!("/api/apps/{}/services/{}", slug, service_id))
        .await?;

    output(state.output_mode, &json!({ "ok": true }), || {
        cli_success(format!("Service {} deleted.", service));
    })?;

    Ok(())
}

pub async fn handle_logs(state: &AppState, slug: &str, service: &str, follow: bool) -> Result<()> {
    let service_id = resolve_service_id(state, slug, service).await?;

    let res = state
        .api_client
        .get_stream(&format!("/api/apps/{}/services/{}/logs", slug, service_id))
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

pub async fn resolve_service_id(state: &AppState, slug: &str, name_or_id: &str) -> Result<String> {
    let services_data = state
        .api_client
        .get(&format!("/api/apps/{}/services", slug))
        .await?;

    let svcs: Vec<Service> = serde_json::from_value(services_data["services"].clone())
        .context("Failed to parse services")?;

    for s in svcs {
        if s.name == name_or_id || s.id == name_or_id {
            return Ok(s.id);
        }
    }

    anyhow::bail!("Service '{}' not found", name_or_id)
}

async fn fetch_default_env(
    state: &AppState,
    kind: &ServiceKind,
) -> Result<std::collections::HashMap<String, String>> {
    let kinds_data = state
        .api_client
        .get("/api/services/kinds")
        .await
        .context("Failed to fetch supported service kinds")?;

    let kinds = kinds_data["kinds"].as_array().cloned().unwrap_or_default();

    let kind_str = kind.to_string();
    for k in kinds {
        if k["name"].as_str().unwrap_or("") == kind_str {
            return serde_json::from_value(k["default_env_vars"].clone())
                .context("Failed to parse default env vars for service kind");
        }
    }

    Ok(Default::default())
}
