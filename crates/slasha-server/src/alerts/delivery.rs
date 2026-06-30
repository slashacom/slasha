use std::{process::Stdio, sync::LazyLock};

use regex::Regex;
use reqwest::Client;
use serde_json::json;
use slasha_db::{
    models::alerts::{AlertChannel, AlertChannelConfig, AlertNotificationKind, AlertRule},
    repos::alerts::AlertChannelRepo,
};
use tokio::process::Command;
use tracing::warn;

use super::evaluation::EvaluationResult;

static TEMPLATE_RE: LazyLock<Regex> = LazyLock::new(|| {
    Regex::new(r"\{\{\s*(value|detail|notification_status|alert_kind)\s*\}\}").unwrap()
});

pub fn render_alert_message(
    rule: &AlertRule,
    observation: &EvaluationResult,
    kind: AlertNotificationKind,
    opened_at: Option<chrono::NaiveDateTime>,
) -> String {
    match rule.message_template.as_deref() {
        Some(template) => {
            let current_value = format_value(observation.current_value);
            render_template(
                template,
                &current_value,
                &observation.detail_display,
                &kind.to_string(),
                rule.kind(),
            )
        }
        None => render_default_message(rule, observation, kind, opened_at),
    }
}

fn render_default_message(
    rule: &AlertRule,
    observation: &EvaluationResult,
    kind: AlertNotificationKind,
    opened_at: Option<chrono::NaiveDateTime>,
) -> String {
    let emoji = match kind {
        AlertNotificationKind::Triggered | AlertNotificationKind::Renotified => ":rotating_light:",
        AlertNotificationKind::Resolved => ":white_check_mark:",
    };

    let current_line = observation
        .current_value
        .map(|v| format!("\n> Current: {}", format_value(Some(v))))
        .unwrap_or_default();
    let limit_line = observation
        .threshold_value
        .map(|v| format!("\n> Limit: {}", format_value(Some(v))))
        .unwrap_or_default();

    let now = chrono::Utc::now().naive_utc();
    let duration_line = opened_at
        .map(|oa| format!("\n> Duration: {}", format_duration(oa, now)))
        .unwrap_or_default();

    format!(
        "{emoji} *{name}*{current_line}{limit_line}\n> Detail: {detail}{duration_line}",
        name = rule.name,
        detail = observation.detail_display,
    )
}

fn format_duration(opened_at: chrono::NaiveDateTime, now: chrono::NaiveDateTime) -> String {
    let secs = (now - opened_at).num_seconds().max(0);
    if secs < 60 {
        "less than a minute".to_string()
    } else if secs < 3600 {
        format!("{} min", secs / 60)
    } else {
        let hours = secs / 3600;
        let mins = (secs % 3600) / 60;
        if mins == 0 {
            format!("{hours} hr")
        } else {
            format!("{hours} hr {mins} min")
        }
    }
}

fn format_value(value: Option<f32>) -> String {
    match value {
        Some(v) if v.fract() == 0.0 => format!("{v:.0}"),
        Some(v) => format!("{v:.1}"),
        None => String::new(),
    }
}

pub async fn deliver_alert(
    db_pool: &slasha_db::DbPool,
    rule: &AlertRule,
    observation: &EvaluationResult,
    kind: AlertNotificationKind,
    message: &str,
    http_client: &Client,
) {
    if let Some(url) = rule.direct_webhook_url.as_deref()
        && let Err(err) = post_webhook(http_client, url, message).await
    {
        warn!(
            target: "slasha::alerts",
            rule_id = %rule.id,
            error = ?err,
            "failed to deliver alert to direct webhook",
        );
    }

    if let Some(command) = rule.shell_command.as_deref()
        && let Err(err) = run_shell_command(rule, observation, kind, command).await
    {
        warn!(
            target: "slasha::alerts",
            rule_id = %rule.id,
            error = ?err,
            "failed to run alert shell command",
        );
    }

    let channels =
        match AlertChannelRepo::list_enabled_by_ids(db_pool, rule.channel_ids.clone()).await {
            Ok(channels) => channels,
            Err(err) => {
                warn!(
                    target: "slasha::alerts",
                    rule_id = %rule.id,
                    error = ?err,
                    "failed to load alert channels",
                );
                return;
            }
        };

    for channel in channels {
        if let Err(err) = deliver_channel(&channel, message, http_client).await {
            warn!(
                target: "slasha::alerts",
                rule_id = %rule.id,
                channel_id = %channel.id,
                error = ?err,
                "failed to deliver alert to channel",
            );
        }
    }
}

async fn deliver_channel(
    channel: &AlertChannel,
    message: &str,
    http_client: &Client,
) -> anyhow::Result<()> {
    match &channel.config {
        AlertChannelConfig::Slack { webhook_url } => {
            post_webhook(http_client, webhook_url, message).await
        }
        AlertChannelConfig::Telegram { bot_token, chat_id } => {
            let url = format!("https://api.telegram.org/bot{bot_token}/sendMessage");
            let response = http_client
                .post(&url)
                .json(&json!({ "chat_id": chat_id, "text": message }))
                .send()
                .await?;
            if !response.status().is_success() {
                return Err(anyhow::anyhow!(
                    "telegram api returned non-success status {}",
                    response.status()
                ));
            }
            Ok(())
        }
    }
}

async fn post_webhook(client: &Client, url: &str, message: &str) -> anyhow::Result<()> {
    let response = client
        .post(url)
        .json(&json!({ "text": message }))
        .send()
        .await?;
    if !response.status().is_success() {
        return Err(anyhow::anyhow!(
            "webhook returned non-success status {}",
            response.status()
        ));
    }

    Ok(())
}

async fn run_shell_command(
    rule: &AlertRule,
    observation: &EvaluationResult,
    kind: AlertNotificationKind,
    command: &str,
) -> anyhow::Result<()> {
    let output = Command::new("sh")
        .arg("-lc")
        .arg(command)
        .env("SLASHA_ALERT_RULE_NAME", &rule.name)
        .env("SLASHA_ALERT_KIND", rule.kind())
        .env("SLASHA_ALERT_STATUS", kind.to_string())
        .env(
            "SLASHA_ALERT_VALUE",
            format_value(observation.current_value),
        )
        .env("SLASHA_ALERT_DETAIL", &observation.detail_display)
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .output()
        .await?;

    if !output.status.success() {
        return Err(anyhow::anyhow!(
            "shell command exited with status {}: {}",
            output.status,
            String::from_utf8_lossy(&output.stderr)
        ));
    }

    Ok(())
}

pub fn render_template(
    template: &str,
    current_value: &str,
    detail: &str,
    notification_status: &str,
    alert_kind: &str,
) -> String {
    TEMPLATE_RE
        .replace_all(template, |caps: &regex::Captures| match &caps[1] {
            "value" => current_value,
            "detail" => detail,
            "notification_status" => notification_status,
            "alert_kind" => alert_kind,
            _ => unreachable!(),
        })
        .into_owned()
}
