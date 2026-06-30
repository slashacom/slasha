use axum::{
    Json, Router,
    extract::{Path, State},
    response::IntoResponse,
    routing::{get, post},
};
use chrono::Utc;
use serde::Deserialize;
use slasha_db::{
    models::alerts::{AlertChannel, AlertChannelConfig, AlertRule, AlertRuleConfig},
    repos::{
        alerts::{AlertChannelRepo, AlertIncidentRepo, AlertNotificationRepo, AlertRuleRepo},
        cron::CronJobRepo,
    },
};
use uuid::Uuid;

use crate::{
    error::{HttpError, HttpResult},
    state::{AppState, Storage},
};

const DEFAULT_ALERT_COOLDOWN_SECS: i32 = 900;

pub fn router() -> Router<AppState> {
    Router::new()
        .route("/channels", get(list_channels).post(create_channel))
        .route(
            "/channels/{id}",
            get(get_channel).put(update_channel).delete(delete_channel),
        )
        .route("/channels/{id}/test", post(test_channel))
        .route("/rules", get(list_rules).post(create_rule))
        .route(
            "/rules/{id}",
            get(get_rule).put(update_rule).delete(delete_rule),
        )
        .route("/incidents", get(list_incidents))
        .route("/incidents/open", get(list_open_incidents))
        .route("/incidents/{id}", get(get_incident))
        .route(
            "/incidents/{id}/notifications",
            get(list_incident_notifications),
        )
        .route("/notifications", get(list_notifications))
        .route("/crons", get(list_all_crons))
}

async fn list_all_crons(State(storage): State<Storage>) -> HttpResult<impl IntoResponse> {
    let crons = CronJobRepo::list_all(&storage.db_pool).await?;
    Ok(Json(serde_json::json!({ "crons": crons })))
}

#[derive(Deserialize)]
struct ChannelInput {
    name: String,
    config: AlertChannelConfig,
    enabled: bool,
}

#[derive(Deserialize)]
struct RuleInput {
    name: String,
    config: AlertRuleConfig,
    channel_ids: Vec<String>,
    direct_webhook_url: Option<String>,
    message_template: Option<String>,
    shell_command: Option<String>,
    enabled: bool,
    cooldown_secs: Option<i32>,
}

async fn list_channels(State(storage): State<Storage>) -> HttpResult<impl IntoResponse> {
    let channels = AlertChannelRepo::list(&storage.db_pool).await?;
    Ok(Json(serde_json::json!({ "channels": channels })))
}

async fn get_channel(
    State(storage): State<Storage>,
    Path(id): Path<String>,
) -> HttpResult<impl IntoResponse> {
    let channel = AlertChannelRepo::find_by_id(&storage.db_pool, &id).await?;
    Ok(Json(serde_json::json!({ "channel": channel })))
}

async fn create_channel(
    State(storage): State<Storage>,
    Json(payload): Json<ChannelInput>,
) -> HttpResult<impl IntoResponse> {
    validate_channel_input(&payload)?;
    let now = Utc::now().naive_utc();
    let channel = AlertChannel {
        id: Uuid::new_v4().to_string(),
        name: payload.name.trim().to_string(),
        config: payload.config,
        enabled: payload.enabled,
        created_at: now,
        updated_at: now,
    };

    let channel = AlertChannelRepo::create(&storage.db_pool, channel).await?;
    Ok(Json(serde_json::json!({ "channel": channel })))
}

async fn update_channel(
    State(storage): State<Storage>,
    Path(id): Path<String>,
    Json(payload): Json<ChannelInput>,
) -> HttpResult<impl IntoResponse> {
    validate_channel_input(&payload)?;
    let now = Utc::now().naive_utc();
    let channel_id = id;
    let channel = AlertChannel {
        id: channel_id.clone(),
        name: payload.name.trim().to_string(),
        config: payload.config,
        enabled: payload.enabled,
        created_at: now,
        updated_at: now,
    };

    let channel = AlertChannelRepo::update(&storage.db_pool, &channel_id, channel).await?;
    Ok(Json(serde_json::json!({ "channel": channel })))
}

async fn delete_channel(
    State(storage): State<Storage>,
    Path(id): Path<String>,
) -> HttpResult<impl IntoResponse> {
    AlertChannelRepo::delete(&storage.db_pool, &id).await?;
    Ok(Json(serde_json::json!({ "deleted": true })))
}

async fn test_channel(
    State(storage): State<Storage>,
    Path(id): Path<String>,
) -> HttpResult<impl IntoResponse> {
    let channel = AlertChannelRepo::find_by_id(&storage.db_pool, &id).await?;
    let http_client = reqwest::Client::new();
    let message = "This is a test message from Slasha. Your alert channel is configured correctly!";

    crate::alerts::delivery::deliver_channel(&channel, message, &http_client)
        .await
        .map_err(|e| HttpError::bad_request(format!("Failed to send test message: {e}")))?;

    Ok(Json(serde_json::json!({ "success": true })))
}

async fn list_rules(State(storage): State<Storage>) -> HttpResult<impl IntoResponse> {
    let rules = AlertRuleRepo::list(&storage.db_pool).await?;
    Ok(Json(serde_json::json!({ "rules": rules })))
}

async fn get_rule(
    State(storage): State<Storage>,
    Path(id): Path<String>,
) -> HttpResult<impl IntoResponse> {
    let rule = AlertRuleRepo::find_by_id(&storage.db_pool, &id).await?;
    Ok(Json(serde_json::json!({ "rule": rule })))
}

async fn create_rule(
    State(storage): State<Storage>,
    Json(payload): Json<RuleInput>,
) -> HttpResult<impl IntoResponse> {
    validate_rule_input(&payload)?;
    let now = Utc::now().naive_utc();
    let rule = AlertRule {
        id: Uuid::new_v4().to_string(),
        name: payload.name.trim().to_string(),
        config: payload.config,
        channel_ids: payload.channel_ids,
        direct_webhook_url: payload.direct_webhook_url,
        message_template: payload.message_template,
        shell_command: payload.shell_command,
        enabled: payload.enabled,
        cooldown_secs: payload.cooldown_secs.unwrap_or(DEFAULT_ALERT_COOLDOWN_SECS),
        created_at: now,
        updated_at: now,
    };

    let rule = AlertRuleRepo::create(&storage.db_pool, rule).await?;
    Ok(Json(serde_json::json!({ "rule": rule })))
}

async fn update_rule(
    State(storage): State<Storage>,
    Path(id): Path<String>,
    Json(payload): Json<RuleInput>,
) -> HttpResult<impl IntoResponse> {
    validate_rule_input(&payload)?;
    let now = Utc::now().naive_utc();
    let rule_id = id;
    let rule = AlertRule {
        id: rule_id.clone(),
        name: payload.name.trim().to_string(),
        config: payload.config,
        channel_ids: payload.channel_ids,
        direct_webhook_url: payload.direct_webhook_url,
        message_template: payload.message_template,
        shell_command: payload.shell_command,
        enabled: payload.enabled,
        cooldown_secs: payload.cooldown_secs.unwrap_or(DEFAULT_ALERT_COOLDOWN_SECS),
        created_at: now,
        updated_at: now,
    };

    let rule = AlertRuleRepo::update(&storage.db_pool, &rule_id, rule).await?;
    Ok(Json(serde_json::json!({ "rule": rule })))
}

async fn delete_rule(
    State(storage): State<Storage>,
    Path(id): Path<String>,
) -> HttpResult<impl IntoResponse> {
    AlertRuleRepo::delete(&storage.db_pool, &id).await?;
    Ok(Json(serde_json::json!({ "deleted": true })))
}

async fn list_incidents(State(storage): State<Storage>) -> HttpResult<impl IntoResponse> {
    let incidents = AlertIncidentRepo::list(&storage.db_pool).await?;
    Ok(Json(serde_json::json!({ "incidents": incidents })))
}

async fn list_open_incidents(State(storage): State<Storage>) -> HttpResult<impl IntoResponse> {
    let incidents = AlertIncidentRepo::list_open(&storage.db_pool).await?;
    Ok(Json(serde_json::json!({ "incidents": incidents })))
}

async fn get_incident(
    State(storage): State<Storage>,
    Path(id): Path<String>,
) -> HttpResult<impl IntoResponse> {
    let incident = AlertIncidentRepo::find_by_id(&storage.db_pool, &id).await?;
    Ok(Json(serde_json::json!({ "incident": incident })))
}

async fn list_incident_notifications(
    State(storage): State<Storage>,
    Path(id): Path<String>,
) -> HttpResult<impl IntoResponse> {
    let incident = AlertIncidentRepo::find_by_id(&storage.db_pool, &id).await?;
    let notifications =
        AlertNotificationRepo::list_for_incident(&storage.db_pool, &incident.id).await?;
    Ok(Json(
        serde_json::json!({ "incident": incident, "notifications": notifications }),
    ))
}

async fn list_notifications(State(storage): State<Storage>) -> HttpResult<impl IntoResponse> {
    let notifications = AlertNotificationRepo::list(&storage.db_pool).await?;
    Ok(Json(serde_json::json!({ "notifications": notifications })))
}

fn validate_channel_input(payload: &ChannelInput) -> HttpResult<()> {
    if payload.name.trim().is_empty() {
        return Err(HttpError::bad_request("Alert channel name cannot be empty"));
    }

    match &payload.config {
        AlertChannelConfig::Slack { webhook_url } => {
            if webhook_url.trim().is_empty() {
                return Err(HttpError::bad_request("Slack webhook URL cannot be empty"));
            }
        }
        AlertChannelConfig::Telegram { bot_token, chat_id } => {
            if bot_token.trim().is_empty() {
                return Err(HttpError::bad_request("Telegram bot token cannot be empty"));
            }
            if chat_id.trim().is_empty() {
                return Err(HttpError::bad_request("Telegram chat id cannot be empty"));
            }
        }
    }

    Ok(())
}

fn validate_rule_input(payload: &RuleInput) -> HttpResult<()> {
    if payload.name.trim().is_empty() {
        return Err(HttpError::bad_request("Alert rule name cannot be empty"));
    }

    for channel_id in &payload.channel_ids {
        if channel_id.trim().is_empty() {
            return Err(HttpError::bad_request("Alert channel id cannot be empty"));
        }
    }

    if let Some(url) = payload.direct_webhook_url.as_deref()
        && url.trim().is_empty()
    {
        return Err(HttpError::bad_request("Webhook URL cannot be empty"));
    }

    if let Some(command) = payload.shell_command.as_deref()
        && command.trim().is_empty()
    {
        return Err(HttpError::bad_request("Shell command cannot be empty"));
    }

    if let Some(template) = payload.message_template.as_deref()
        && template.trim().is_empty()
    {
        return Err(HttpError::bad_request("Message template cannot be empty"));
    }

    match &payload.config {
        AlertRuleConfig::ServerCpu { threshold_percent }
        | AlertRuleConfig::ServerMemory { threshold_percent } => {
            validate_percent(*threshold_percent)?
        }
        AlertRuleConfig::ServerLoadAverage { threshold } => {
            if *threshold <= 0.0 {
                return Err(HttpError::bad_request(
                    "Load average threshold must be greater than zero",
                ));
            }
        }
        AlertRuleConfig::AppCpu {
            app_id,
            threshold_percent,
        }
        | AlertRuleConfig::AppMemory {
            app_id,
            threshold_percent,
        } => {
            if app_id.trim().is_empty() {
                return Err(HttpError::bad_request("App id cannot be empty"));
            }
            validate_percent(*threshold_percent)?;
        }
        AlertRuleConfig::DomainTlsExpiry {
            domain,
            days_before,
        } => {
            if domain.trim().is_empty() {
                return Err(HttpError::bad_request("Domain cannot be empty"));
            }
            if *days_before < 0 {
                return Err(HttpError::bad_request(
                    "TLS expiry threshold cannot be negative",
                ));
            }
        }
        AlertRuleConfig::DomainDnsMisconfigured { domain } => {
            if domain.trim().is_empty() {
                return Err(HttpError::bad_request("Domain cannot be empty"));
            }
        }
        AlertRuleConfig::AppHealthCheck { app_id, url } => {
            if app_id.trim().is_empty() {
                return Err(HttpError::bad_request("App id cannot be empty"));
            }

            if url.trim().is_empty() {
                return Err(HttpError::bad_request("Health check URL cannot be empty"));
            }
        }
        AlertRuleConfig::CronFailed { cron_job_id } => {
            if cron_job_id.trim().is_empty() {
                return Err(HttpError::bad_request("Cron job is required"));
            }
        }
    }

    if let Some(cooldown_secs) = payload.cooldown_secs
        && cooldown_secs <= 0
    {
        return Err(HttpError::bad_request(
            "Cooldown seconds must be greater than zero",
        ));
    }
    Ok(())
}

fn validate_percent(value: f32) -> HttpResult<()> {
    if !(0.0..=100.0).contains(&value) {
        return Err(HttpError::bad_request(
            "Percentage threshold must be between 0 and 100",
        ));
    }

    Ok(())
}
