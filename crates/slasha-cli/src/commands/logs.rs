use std::fmt::Write;

use anyhow::Result;
use colored::Colorize;
use eventsource_stream::Eventsource;
use futures_util::StreamExt;
use slasha_db::models::logs::{LogRecord, LogStream};

use crate::{
    clap_app::LogArgs,
    commands::responses::LogsResponse,
    http::ApiClient,
    output::{cli_error, cli_info},
};

/// Fetches historical logs or streams live logs based on [`LogArgs`].
///
/// # Arguments
///
/// * `client` - Reference to the API client ([`ApiClient`]).
/// * `resource_path` - Base API endpoint path (e.g. `/api/apps/app-slug/deployments/dep_123`).
/// * `target_label` - Human-readable label for the target application or service (e.g. `epic-owl-49a`).
/// * `args` - Log command filter and paging flags ([`LogArgs`]).
pub async fn display_logs(
    client: &ApiClient,
    resource_path: &str,
    target_label: &str,
    args: &LogArgs,
) -> Result<()> {
    let filter_opts = LogFilterOptions {
        search: args.search.as_deref(),
        prefix: args.prefix.as_deref(),
        stream: args.stream,
    };

    if args.follow {
        let res = client
            .get_stream(&format!("{}/stream", resource_path))
            .await?;

        stream_logs(res, filter_opts).await?;
    } else {
        let mut query_params = vec![format!("limit={}", args.limit)];

        if let Some(ref s) = args.search {
            let encoded: String = url::form_urlencoded::byte_serialize(s.as_bytes()).collect();
            query_params.push(format!("search={}", encoded));
        }
        if let Some(ref p) = args.prefix {
            let encoded: String = url::form_urlencoded::byte_serialize(p.as_bytes()).collect();
            query_params.push(format!("prefix={}", encoded));
        }
        if let Some(st) = args.stream {
            let encoded: String =
                url::form_urlencoded::byte_serialize(st.to_string().as_bytes()).collect();
            query_params.push(format!("stream={}", encoded));
        }

        let data: LogsResponse = client
            .get(&format!(
                "{}/logs?{}",
                resource_path,
                query_params.join("&")
            ))
            .await?;

        page_logs(target_label, &data.logs)?;
    }

    Ok(())
}

/// Formats a log record into a colorized single line string.
fn format_log_record(rec: &LogRecord) -> String {
    let timestamp = rec
        .timestamp
        .format("%Y-%m-%d %H:%M:%S")
        .to_string()
        .dimmed();

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
async fn stream_logs(res: reqwest::Response, filter: LogFilterOptions<'_>) -> Result<()> {
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
                    .filter(|r| filter.matches(r))
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
