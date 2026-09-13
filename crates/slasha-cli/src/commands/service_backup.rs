use std::io::IsTerminal;

use anyhow::{Context, Result};
use colored::Colorize;
use futures_util::StreamExt;
use serde::{Deserialize, Serialize};
use serde_json::json;
use slasha_db::models::service_backup::{
    ServiceBackup, ServiceBackupConfig, ServiceBackupStatus, ServiceRestoreStatus,
};
use tokio::{
    fs::File,
    io::{AsyncWriteExt, stdout},
};

use crate::{
    clap_app::ServiceBackupCommand,
    commands::{
        resolve::{resolve_backup, resolve_service_id},
        responses::OkResponse,
    },
    http::ApiClient,
    output::{
        cli_info, cli_label, cli_success, confirm_action, format_local_datetime,
        format_local_datetime_secs, print_table, spinner,
    },
};

#[derive(Deserialize, Serialize)]
struct ServiceBackupsResponse {
    backups: Vec<ServiceBackup>,
}

#[derive(Deserialize, Serialize)]
struct ServiceBackupItemResponse {
    backup: ServiceBackup,
}

#[derive(Deserialize, Serialize)]
struct ServiceBackupConfigResponse {
    config: Option<ServiceBackupConfig>,
}

/// Dispatches service backup subcommands for an application service.
///
/// # Arguments
///
/// * `client` - Reference to the API client ([`ApiClient`]).
/// * `slug` - Target application slug.
/// * `service` - Service name.
/// * `cmd` - Target backup subcommand ([`ServiceBackupCommand`]).
pub async fn dispatch(
    client: &ApiClient,
    slug: &str,
    service: &str,
    cmd: ServiceBackupCommand,
) -> Result<()> {
    let service_id = resolve_service_id(client, slug, service).await?;
    match cmd {
        ServiceBackupCommand::List => handle_list(client, slug, &service_id, service).await,
        ServiceBackupCommand::Trigger => handle_trigger(client, slug, &service_id, service).await,
        ServiceBackupCommand::Download { backup, file } => {
            handle_download(client, slug, &service_id, service, backup.as_deref(), file).await
        }
        ServiceBackupCommand::Restore { backup, yes } => {
            handle_restore(client, slug, &service_id, service, backup.as_deref(), yes).await
        }
        ServiceBackupCommand::Delete { backup, yes } => {
            handle_delete(client, slug, &service_id, service, &backup, yes).await
        }
        ServiceBackupCommand::Config => handle_config(client, slug, &service_id, service).await,
    }
}

async fn handle_list(
    client: &ApiClient,
    slug: &str,
    service_id: &str,
    service_name: &str,
) -> Result<()> {
    let res: ServiceBackupsResponse = client
        .get(&format!(
            "/api/apps/{}/services/{}/backups",
            slug, service_id
        ))
        .await?;

    if res.backups.is_empty() {
        cli_info(format!(
            "No backups found for service '{}'. Run `slasha services backups {} trigger` to create one.",
            service_name, service_name
        ));
    } else {
        let mut rows: Vec<Vec<String>> = res
            .backups
            .iter()
            .map(|b| {
                let restored = b
                    .last_restored_at
                    .map(format_local_datetime)
                    .unwrap_or_else(|| "-".to_string());
                vec![
                    b.file_name.clone(),
                    format_backup_status(b.status, b.restore_status, b.error.is_some()),
                    b.trigger_kind.to_string(),
                    format_file_size(b.file_size),
                    format_storage(b.stored_locally, b.s3_storage_id.is_some()),
                    format_local_datetime(b.created_at),
                    restored,
                ]
            })
            .collect();

        rows.sort_by(|a, b| b[5].cmp(&a[5]));
        print_table(
            &[
                "NAME", "STATUS", "TRIGGER", "SIZE", "STORAGE", "CREATED", "RESTORED",
            ],
            rows,
        );
    }

    Ok(())
}

async fn handle_trigger(
    client: &ApiClient,
    slug: &str,
    service_id: &str,
    service_name: &str,
) -> Result<()> {
    let res: ServiceBackupItemResponse = client
        .post(
            &format!("/api/apps/{}/services/{}/backups", slug, service_id),
            &json!({}),
        )
        .await?;

    cli_success("Backup triggered.");
    cli_info(format!(
        "\nCheck status: slasha services backups {} list",
        service_name
    ));
    cli_info(format!(
        "Download when ready: slasha services backups {} download {}",
        service_name, res.backup.file_name
    ));

    Ok(())
}

async fn handle_download(
    client: &ApiClient,
    slug: &str,
    service_id: &str,
    service_name: &str,
    backup_ref: Option<&str>,
    file_path: Option<String>,
) -> Result<()> {
    let backup = resolve_backup(client, slug, service_id, service_name, backup_ref).await?;

    if backup.status == ServiceBackupStatus::Running {
        anyhow::bail!(
            "Backup '{}' is currently running. Please wait for it to complete.",
            backup.file_name
        );
    }

    if backup.status != ServiceBackupStatus::Succeeded {
        anyhow::bail!(
            "Cannot download failed backup '{}': {}",
            backup.file_name,
            backup.error.as_deref().unwrap_or("unknown error")
        );
    }

    let res = client
        .get_stream(&format!(
            "/api/apps/{}/services/{}/backups/{}/download",
            slug, service_id, backup.id
        ))
        .await?;

    let mut stream = res.bytes_stream();
    let is_tty = std::io::stdout().is_terminal();

    let target_path = match file_path {
        Some(path) => Some(path),
        None if is_tty => Some(backup.file_name.clone()),
        None => None,
    };

    match target_path {
        Some(path) => {
            let mut file = File::create(&path)
                .await
                .with_context(|| format!("Failed to create file: {}", path))?;

            cli_info(format!(
                "Downloading {} ({})...",
                backup.file_name,
                format_file_size(backup.file_size)
            ));
            let mut total: u64 = 0;

            while let Some(chunk) = stream.next().await {
                let chunk = chunk.context("Stream error")?;
                total += chunk.len() as u64;
                file.write_all(&chunk).await.context("Write error")?;
            }

            file.flush().await.context("Flush error")?;
            cli_success(format!(
                "Done. {} written to {}.",
                format_file_size(total as i64),
                path
            ));
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

async fn handle_restore(
    client: &ApiClient,
    slug: &str,
    service_id: &str,
    service_name: &str,
    backup_ref: Option<&str>,
    yes: bool,
) -> Result<()> {
    let backup = resolve_backup(client, slug, service_id, service_name, backup_ref).await?;

    if backup.status != ServiceBackupStatus::Succeeded {
        anyhow::bail!(
            "Cannot restore from backup '{}' because it is not in completed state ({:?})",
            backup.file_name,
            backup.status
        );
    }

    if !confirm_action(
        yes,
        &format!(
            "Restore database for service '{}' from backup '{}'? Current data will be replaced.",
            service_name.yellow(),
            backup.file_name.cyan()
        ),
    )? {
        return Ok(());
    }

    let _spin = spinner("Triggering database restore...");
    let _: OkResponse = client
        .post(
            &format!(
                "/api/apps/{}/services/{}/backups/{}/restore",
                slug, service_id, backup.id
            ),
            &json!({}),
        )
        .await?;

    cli_success(format!(
        "Database restore triggered for service '{}' from backup '{}'.",
        service_name, backup.file_name
    ));

    Ok(())
}

async fn handle_delete(
    client: &ApiClient,
    slug: &str,
    service_id: &str,
    service_name: &str,
    backup_ref: &str,
    yes: bool,
) -> Result<()> {
    let backup = resolve_backup(client, slug, service_id, service_name, Some(backup_ref)).await?;

    if !confirm_action(
        yes,
        &format!(
            "Delete backup '{}' for service '{}'?",
            backup.file_name.red(),
            service_name
        ),
    )? {
        return Ok(());
    }

    let _spin = spinner("Deleting backup...");
    let _: OkResponse = client
        .delete(&format!(
            "/api/apps/{}/services/{}/backups/{}",
            slug, service_id, backup.id
        ))
        .await?;

    cli_success(format!("Backup '{}' deleted.", backup.file_name));

    Ok(())
}

async fn handle_config(
    client: &ApiClient,
    slug: &str,
    service_id: &str,
    service_name: &str,
) -> Result<()> {
    let res: ServiceBackupConfigResponse = client
        .get(&format!(
            "/api/apps/{}/services/{}/backup-config",
            slug, service_id
        ))
        .await?;

    if let Some(cfg) = res.config {
        cli_info(format!(
            "Backup configuration for service '{}':",
            service_name
        ));
        cli_label(
            "Enabled",
            if cfg.enabled {
                "true".green().to_string()
            } else {
                "false".dimmed().to_string()
            },
        );
        cli_label("Schedule", &cfg.schedule);
        cli_label("Timezone", &cfg.timezone);
        cli_label("Retention Count", cfg.retention_count.to_string());
        cli_label("Keep Local", cfg.keep_local.to_string());
        cli_label(
            "S3 Storage ID",
            cfg.s3_storage_id.as_deref().unwrap_or("(none)"),
        );
        cli_label(
            "Last Run",
            cfg.last_run_at
                .map(format_local_datetime_secs)
                .unwrap_or_else(|| "(never)".into()),
        );
        cli_label(
            "Next Run",
            cfg.next_run_at
                .map(format_local_datetime_secs)
                .unwrap_or_else(|| "(not scheduled)".into()),
        );
    } else {
        cli_info(
            "No backup configuration found. Configure backup storage destinations and schedules in the web dashboard.",
        );
    }

    Ok(())
}

fn format_backup_status(
    status: ServiceBackupStatus,
    restore_status: ServiceRestoreStatus,
    has_error: bool,
) -> String {
    if restore_status == ServiceRestoreStatus::Restoring {
        "Restoring".yellow().to_string()
    } else if restore_status == ServiceRestoreStatus::Failed {
        "Restore Failed".red().to_string()
    } else {
        match status {
            ServiceBackupStatus::Running => "Running".cyan().to_string(),
            ServiceBackupStatus::Succeeded => {
                if has_error {
                    "S3 Failed".yellow().to_string()
                } else {
                    "Succeeded".green().to_string()
                }
            }
            ServiceBackupStatus::Failed => "Failed".red().to_string(),
        }
    }
}

fn format_storage(stored_locally: bool, has_s3: bool) -> String {
    match (stored_locally, has_s3) {
        (true, true) => "Local + S3".to_string(),
        (true, false) => "Local".to_string(),
        (false, true) => "S3".to_string(),
        (false, false) => "-".dimmed().to_string(),
    }
}

fn format_file_size(bytes: i64) -> String {
    if bytes <= 0 {
        return "0 B".to_string();
    }
    const UNITS: &[&str] = &["B", "KB", "MB", "GB", "TB"];
    let mut size = bytes as f64;
    let mut unit_idx = 0;
    while size >= 1024.0 && unit_idx < UNITS.len() - 1 {
        size /= 1024.0;
        unit_idx += 1;
    }
    if unit_idx == 0 {
        format!("{} B", bytes)
    } else {
        format!("{:.1} {}", size, UNITS[unit_idx])
    }
}
