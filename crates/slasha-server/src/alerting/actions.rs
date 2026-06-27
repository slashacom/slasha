//! Delivery of a fired alert through its configured action.
//!
//! Three action types:
//! - `channel`         — deliver via a reusable [`Channel`] (Slack, Telegram).
//! - `webhook`         — POST `{ "text": <message> }` to an inline URL.
//! - `execute_program` — run an admin-authored shell command with the event
//!   exposed as environment variables.

use std::{sync::OnceLock, time::Duration};

use anyhow::{Context, Result, anyhow};
use serde::Deserialize;
use slasha_db::{
    DbPool,
    models::{alert_rule::AlertRule, channel::Channel},
    repos::channel::ChannelRepo,
};

use super::AlertEvent;

const DEFAULT_COMMAND_TIMEOUT_SECS: u64 = 30;

fn http_client() -> &'static reqwest::Client {
    static CLIENT: OnceLock<reqwest::Client> = OnceLock::new();
    CLIENT.get_or_init(reqwest::Client::new)
}

#[derive(Debug, Deserialize)]
struct ChannelActionConfig {
    channel_id: String,
}

#[derive(Debug, Deserialize)]
struct WebhookActionConfig {
    url: String,
}

#[derive(Debug, Deserialize)]
struct CommandActionConfig {
    command: String,
    #[serde(default)]
    timeout_secs: Option<u64>,
}

#[derive(Debug, Deserialize)]
struct SlackConfig {
    webhook_url: String,
}

#[derive(Debug, Deserialize)]
struct TelegramConfig {
    bot_token: String,
    chat_id: String,
}

/// Deliver a rendered alert message according to the rule's action type.
pub async fn execute(
    pool: &DbPool,
    rule: &AlertRule,
    event: &AlertEvent,
    message: &str,
) -> Result<()> {
    match rule.action_type.as_str() {
        "channel" => {
            let cfg: ChannelActionConfig = serde_json::from_str(&rule.action_config)
                .context("invalid channel action config")?;
            let channel = ChannelRepo::get(pool, &cfg.channel_id)
                .await
                .context("channel referenced by rule not found")?;
            send_via_channel(&channel, message).await
        }
        "webhook" => {
            let cfg: WebhookActionConfig = serde_json::from_str(&rule.action_config)
                .context("invalid webhook action config")?;
            post_json(&cfg.url, &serde_json::json!({ "text": message })).await
        }
        "execute_program" => {
            let cfg: CommandActionConfig = serde_json::from_str(&rule.action_config)
                .context("invalid execute_program action config")?;
            run_command(&cfg, event, message).await
        }
        other => Err(anyhow!("unknown action type: {other}")),
    }
}

/// Send a message through a configured channel. Also used by the channel
/// "test" endpoint.
pub async fn send_via_channel(channel: &Channel, message: &str) -> Result<()> {
    match channel.kind.as_str() {
        "slack" => {
            let cfg: SlackConfig =
                serde_json::from_str(&channel.config).context("invalid slack channel config")?;
            post_json(&cfg.webhook_url, &serde_json::json!({ "text": message })).await
        }
        "telegram" => {
            let cfg: TelegramConfig =
                serde_json::from_str(&channel.config).context("invalid telegram channel config")?;
            let url = format!("https://api.telegram.org/bot{}/sendMessage", cfg.bot_token);
            post_json(
                &url,
                &serde_json::json!({ "chat_id": cfg.chat_id, "text": message }),
            )
            .await
        }
        other => Err(anyhow!("unknown channel kind: {other}")),
    }
}

async fn post_json(url: &str, body: &serde_json::Value) -> Result<()> {
    let response = http_client()
        .post(url)
        .json(body)
        .send()
        .await
        .context("failed to send alert request")?;

    if !response.status().is_success() {
        let status = response.status();
        let text = response.text().await.unwrap_or_default();
        return Err(anyhow!("alert endpoint returned {status}: {text}"));
    }

    Ok(())
}

async fn run_command(cfg: &CommandActionConfig, event: &AlertEvent, message: &str) -> Result<()> {
    let timeout = Duration::from_secs(cfg.timeout_secs.unwrap_or(DEFAULT_COMMAND_TIMEOUT_SECS));

    let mut command = tokio::process::Command::new("sh");
    command
        .arg("-c")
        .arg(&cfg.command)
        .env("SLASHA_EVENT", &event.event)
        .env("SLASHA_TARGET", &event.target)
        .env("SLASHA_TITLE", &event.title)
        .env("SLASHA_VALUE", format!("{:.2}", event.value))
        .env("SLASHA_UNIT", &event.unit)
        .env("SLASHA_DETAIL", &event.detail)
        .env("SLASHA_MESSAGE", message)
        .kill_on_drop(true);

    let output = tokio::time::timeout(timeout, command.output())
        .await
        .map_err(|_| anyhow!("command timed out after {}s", timeout.as_secs()))?
        .context("failed to spawn command")?;

    if !output.status.success() {
        let stderr = String::from_utf8_lossy(&output.stderr);
        return Err(anyhow!(
            "command exited with {}: {}",
            output.status,
            stderr.trim()
        ));
    }

    Ok(())
}
