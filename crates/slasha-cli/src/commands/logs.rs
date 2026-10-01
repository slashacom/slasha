use std::{collections::HashSet, fmt::Write, time::Duration};

use anyhow::Result;
use colored::Colorize;
use eventsource_stream::Eventsource;
use futures_util::StreamExt;
use slasha_db::{
    deployment::{Deployment, DeploymentStatus},
    models::logs::{LogRecord, LogStream},
};

use crate::{
    clap_app::LogArgs,
    http::ApiClient,
    output::{cli_error, cli_info, format_local_datetime_secs},
};

const STATUS_POLL_INTERVAL: Duration = Duration::from_secs(3);

#[derive(serde::Deserialize)]
struct LogsResponse {
    logs: Vec<LogRecord>,
}

#[derive(serde::Deserialize)]
struct DeploymentResponse {
    deployment: Deployment,
}

/// What the logs belong to, which decides when following them stops.
#[derive(Clone, Copy, PartialEq, Eq)]
pub enum LogSource {
    /// Follows until interrupted.
    Service,
    /// Stops following once the deployment has failed or been stopped; a
    /// deployment that is already finished is not followed at all.
    Deployment,
}

/// Prints a resource's stored logs, then follows live ones when asked to.
///
/// Following subscribes to the live stream before reading the history, so a
/// line written in between arrives on the stream and is printed once.
///
/// # Arguments
///
/// * `client` - Reference to the API client ([`ApiClient`]).
/// * `resource_path` - Base API endpoint path (e.g. `/api/apps/app-slug/deployments/dep_123`).
/// * `target_label` - Human-readable label for the target application or service (e.g. `epic-owl-49a`).
/// * `args` - Log command filter and paging flags ([`LogArgs`]).
/// * `source` - What the logs belong to ([`LogSource`]).
///
/// # Returns
///
/// The deployment's status when following stopped, for a [`LogSource::Deployment`].
pub async fn display_logs(
    client: &ApiClient,
    resource_path: &str,
    target_label: &str,
    args: &LogArgs,
    source: LogSource,
) -> Result<Option<DeploymentStatus>> {
    if !args.follow {
        let logs = fetch_history(client, resource_path, args).await?;
        page_logs(target_label, &logs)?;
        return Ok(None);
    }

    let filter = LogFilterOptions {
        search: args.search.as_deref(),
        prefix: args.prefix.as_deref(),
        stream: args.stream,
    };

    let live = client
        .get_stream(&format!("{}/stream", resource_path))
        .await?;

    let history = fetch_history(client, resource_path, args).await?;
    let seen: HashSet<String> = history.iter().map(|rec| rec.id.clone()).collect();
    for rec in &history {
        print_log_record(rec);
    }

    if source == LogSource::Service {
        stream_logs(live, filter, &seen).await?;
        return Ok(None);
    }

    let status = fetch_deployment_status(client, resource_path).await?;
    if is_finished(status) {
        if history.is_empty() {
            cli_info("No logs available.");
        }
        cli_info(format!(
            "\nDeployment is {}; nothing more to follow.",
            status
        ));
        return Ok(Some(status));
    }

    let finished = async {
        loop {
            tokio::time::sleep(STATUS_POLL_INTERVAL).await;
            if let Ok(status) = fetch_deployment_status(client, resource_path).await
                && is_finished(status)
            {
                return status;
            }
        }
    };

    tokio::select! {
        res = stream_logs(live, filter, &seen) => {
            res?;
            Ok(None)
        }
        status = finished => {
            cli_info(format!("\nDeployment is {}.", status));
            Ok(Some(status))
        }
    }
}

fn is_finished(status: DeploymentStatus) -> bool {
    matches!(status, DeploymentStatus::Failed | DeploymentStatus::Stopped)
}

async fn fetch_deployment_status(
    client: &ApiClient,
    resource_path: &str,
) -> Result<DeploymentStatus> {
    let res: DeploymentResponse = client.get(resource_path).await?;
    Ok(res.deployment.status)
}

fn history_query(args: &LogArgs) -> String {
    let encode = |value: &str| -> String {
        url::form_urlencoded::byte_serialize(value.as_bytes()).collect()
    };

    let mut query_params = vec![format!("limit={}", args.limit)];

    if let Some(ref s) = args.search {
        query_params.push(format!("search={}", encode(s)));
    }
    if let Some(ref p) = args.prefix {
        query_params.push(format!("prefix={}", encode(p)));
    }
    if let Some(st) = args.stream {
        query_params.push(format!("stream={}", encode(&st.to_string())));
    }

    query_params.join("&")
}

async fn fetch_history(
    client: &ApiClient,
    resource_path: &str,
    args: &LogArgs,
) -> Result<Vec<LogRecord>> {
    let data: LogsResponse = client
        .get(&format!("{}/logs?{}", resource_path, history_query(args)))
        .await?;

    Ok(data.logs)
}

/// Formats a log record into a colorized single line string.
fn format_log_record(rec: &LogRecord) -> String {
    let timestamp = format_local_datetime_secs(rec.timestamp).dimmed();

    let stream = match rec.stream {
        LogStream::Stdout => "[stdout]".green().to_string(),
        LogStream::Stderr => "[stderr]".red().to_string(),
    };

    if let Some(ref p) = rec.prefix {
        let prefix = format!("[{}]", p).cyan();
        format!("{} {} {} {}", timestamp, prefix, stream, rec.message)
    } else {
        format!("{} {} {}", timestamp, stream, rec.message)
    }
}

/// Prints a formatted log record line to stdout.
fn print_log_record(rec: &LogRecord) {
    cli_info(format_log_record(rec));
}

/// Helper struct containing log streaming filter options.
#[derive(Default, Clone)]
struct LogFilterOptions<'a> {
    search: Option<&'a str>,
    prefix: Option<&'a str>,
    stream: Option<LogStream>,
}

impl LogFilterOptions<'_> {
    /// Evaluates if a log record matches all active filter rules.
    fn matches(&self, log: &LogRecord) -> bool {
        if self.stream.is_some_and(|stream| log.stream != stream) {
            return false;
        }

        if let Some(prefix) = self.prefix {
            let record_prefix = log
                .prefix
                .as_ref()
                .map(ToString::to_string)
                .unwrap_or_default();

            if !record_prefix.eq_ignore_ascii_case(prefix)
                && !record_prefix
                    .to_lowercase()
                    .starts_with(&format!("{}.", prefix.to_lowercase()))
            {
                return false;
            }
        }

        if let Some(search) = self.search {
            let search = search.to_lowercase();

            let message_matches = log.message.to_lowercase().contains(&search);
            let prefix_matches = log
                .prefix
                .as_ref()
                .is_some_and(|prefix| prefix.to_string().to_lowercase().contains(&search));

            if !message_matches && !prefix_matches {
                return false;
            }
        }

        true
    }
}

/// Displays a list of log records using the terminal pager or stdout printing.
fn page_logs(target_label: &str, logs: &[LogRecord]) -> Result<()> {
    if logs.is_empty() {
        cli_info("No logs available.");
        return Ok(());
    }

    if !std::io::IsTerminal::is_terminal(&std::io::stdout()) {
        for rec in logs {
            print_log_record(rec);
        }
        return Ok(());
    }

    let mut pager = minus::Pager::new();
    let total = logs.len();
    let line_str = if total == 1 {
        "1 line"
    } else {
        &format!("{} lines", total)
    };
    let prompt = format!(
        "{} ({}) | press 'q' to quit, '/' to search",
        target_label, line_str
    );
    pager.set_prompt(prompt)?;

    for rec in logs {
        writeln!(pager, "{}", format_log_record(rec))?;
    }

    minus::page_all(pager)?;
    Ok(())
}

/// Streams Server-Sent Events (SSE) formatted log records to stdout.
async fn stream_logs(
    res: reqwest::Response,
    filter: LogFilterOptions<'_>,
    seen: &HashSet<String>,
) -> Result<()> {
    let mut stream = res.bytes_stream().eventsource();

    while let Some(event) = stream.next().await {
        match event {
            Ok(event) => {
                let data = event.data.trim();

                if data.is_empty() {
                    continue;
                }

                if let Some(rec) = serde_json::from_str::<LogRecord>(data)
                    .ok()
                    .filter(|r| filter.matches(r) && !seen.contains(&r.id))
                {
                    print_log_record(&rec);
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
