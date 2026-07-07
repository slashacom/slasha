use axum::{
    Json, Router,
    extract::{Path, State},
    response::IntoResponse,
    routing::{get, post},
};
use garde::Validate;
use serde::Deserialize;
use slasha_db::{
    models::alerts::{
        AlertChannelChangeset, AlertChannelConfig as DbAlertChannelConfig, AlertRuleChangeset,
        AlertRuleConfig as DbAlertRuleConfig, NewAlertChannel, NewAlertRule,
    },
    repos::{
        alerts::{AlertChannelRepo, AlertIncidentRepo, AlertNotificationRepo, AlertRuleRepo},
        cron::CronJobRepo,
    },
};

use crate::{
    HttpError, HttpResult,
    alerts::delivery,
    extractors::ValidatedJson,
    routing::api::{
        deserialize::{trim_optional_string, trim_string, trim_string_vec},
        validation::not_empty,
    },
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

#[derive(Deserialize, Validate)]
struct ChannelInput {
    #[serde(deserialize_with = "trim_string")]
    #[garde(custom(not_empty))]
    name: String,
    #[garde(dive)]
    config: ChannelConfigInput,
    #[garde(skip)]
    enabled: bool,
}

#[derive(Deserialize, Validate)]
#[serde(tag = "kind", rename_all = "snake_case")]
enum ChannelConfigInput {
    Slack {
        #[serde(deserialize_with = "trim_string")]
        #[garde(url, prefix("http"))]
        webhook_url: String,
    },
    Discord {
        #[serde(deserialize_with = "trim_string")]
        #[garde(url, prefix("http"))]
        webhook_url: String,
    },
    Telegram {
        #[serde(deserialize_with = "trim_string")]
        #[garde(custom(not_empty), contains(":"))]
        bot_token: String,
        #[serde(deserialize_with = "trim_string")]
        #[garde(custom(not_empty))]
        chat_id: String,
    },
    Email {
        #[serde(deserialize_with = "trim_string")]
        #[garde(custom(not_empty))]
        smtp_host: String,
        #[garde(range(min = 1))]
        smtp_port: u16,
        #[serde(deserialize_with = "trim_string")]
        #[garde(custom(not_empty))]
        smtp_username: String,
        #[serde(deserialize_with = "trim_string")]
        #[garde(custom(not_empty))]
        smtp_password: String,
        #[serde(deserialize_with = "trim_string")]
        #[garde(email)]
        from_address: String,
        #[serde(deserialize_with = "trim_string")]
        #[garde(email)]
        to_address: String,
    },
}

#[derive(Deserialize, Validate)]
struct RuleInput {
    #[serde(deserialize_with = "trim_string")]
    #[garde(custom(not_empty))]
    name: String,
    #[garde(dive)]
    config: RuleConfigInput,
    #[serde(deserialize_with = "trim_string_vec")]
    #[garde(inner(custom(not_empty)))]
    channel_ids: Vec<String>,
    #[serde(default, deserialize_with = "trim_optional_string")]
    #[garde(inner(url, prefix("http")))]
    direct_webhook_url: Option<String>,
    #[serde(default, deserialize_with = "trim_optional_string")]
    #[garde(skip)]
    message_template: Option<String>,
    #[serde(default, deserialize_with = "trim_optional_string")]
    #[garde(skip)]
    shell_command: Option<String>,
    #[garde(skip)]
    enabled: bool,
    #[serde(default = "default_alert_cooldown_secs")]
    #[garde(range(min = 1))]
    cooldown_secs: i32,
}

#[derive(Deserialize, Validate)]
#[serde(tag = "kind", rename_all = "snake_case")]
enum RuleConfigInput {
    ServerCpu {
        #[garde(range(min = 0.0, max = 100.0))]
        threshold_percent: f64,
    },
    ServerMemory {
        #[garde(range(min = 0.0, max = 100.0))]
        threshold_percent: f64,
    },
    ServerLoadAverage {
        #[garde(range(min = 0.0))]
        threshold: f64,
    },
    AppCpu {
        #[serde(deserialize_with = "trim_string")]
        #[garde(custom(not_empty))]
        app_id: String,
        #[garde(range(min = 0.0, max = 100.0))]
        threshold_percent: f64,
    },
    AppMemory {
        #[serde(deserialize_with = "trim_string")]
        #[garde(custom(not_empty))]
        app_id: String,
        #[garde(range(min = 0.0, max = 100.0))]
        threshold_percent: f64,
    },
    DomainTlsExpiry {
        #[serde(deserialize_with = "trim_string")]
        #[garde(custom(not_empty))]
        domain: String,
        #[garde(range(min = 0))]
        days_before: i32,
    },
    DomainDnsMisconfigured {
        #[serde(deserialize_with = "trim_string")]
        #[garde(custom(not_empty))]
        domain: String,
    },
    AppHealthCheck {
        #[serde(deserialize_with = "trim_string")]
        #[garde(custom(not_empty))]
        app_id: String,
        #[serde(deserialize_with = "trim_string")]
        #[garde(url, prefix("http"))]
        health_check_url: String,
    },
    CronFailed {
        #[serde(deserialize_with = "trim_string")]
        #[garde(custom(not_empty))]
        cron_job_id: String,
    },
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
    ValidatedJson(payload): ValidatedJson<ChannelInput>,
) -> HttpResult<impl IntoResponse> {
    let channel = NewAlertChannel {
        name: payload.name,
        config: payload.config.into(),
        enabled: payload.enabled,
    };

    let channel = AlertChannelRepo::create(&storage.db_pool, channel).await?;
    Ok(Json(serde_json::json!({ "channel": channel })))
}

async fn update_channel(
    State(storage): State<Storage>,
    Path(id): Path<String>,
    ValidatedJson(payload): ValidatedJson<ChannelInput>,
) -> HttpResult<impl IntoResponse> {
    let changeset = AlertChannelChangeset {
        name: payload.name,
        config: payload.config.into(),
        enabled: payload.enabled,
    };

    let channel = AlertChannelRepo::update(&storage.db_pool, &id, changeset).await?;
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

    delivery::deliver_channel(
        &channel,
        "This is a test message from Slasha. Your alert channel is configured correctly!",
        &http_client,
    )
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
    ValidatedJson(payload): ValidatedJson<RuleInput>,
) -> HttpResult<impl IntoResponse> {
    let rule = NewAlertRule {
        name: payload.name,
        config: payload.config.into(),
        channel_ids: payload.channel_ids,
        direct_webhook_url: payload.direct_webhook_url,
        message_template: payload.message_template,
        shell_command: payload.shell_command,
        enabled: payload.enabled,
        cooldown_secs: payload.cooldown_secs,
    };

    let rule = AlertRuleRepo::create(&storage.db_pool, rule).await?;
    Ok(Json(serde_json::json!({ "rule": rule })))
}

async fn update_rule(
    State(storage): State<Storage>,
    Path(id): Path<String>,
    ValidatedJson(payload): ValidatedJson<RuleInput>,
) -> HttpResult<impl IntoResponse> {
    let changeset = AlertRuleChangeset {
        name: payload.name,
        config: payload.config.into(),
        channel_ids: payload.channel_ids,
        direct_webhook_url: payload.direct_webhook_url,
        message_template: payload.message_template,
        shell_command: payload.shell_command,
        enabled: payload.enabled,
        cooldown_secs: payload.cooldown_secs,
    };

    let rule = AlertRuleRepo::update(&storage.db_pool, &id, changeset).await?;
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

fn default_alert_cooldown_secs() -> i32 {
    DEFAULT_ALERT_COOLDOWN_SECS
}

// helper macro to impl From for config types
macro_rules! impl_config_conversion {
    ($source:ident => $target:ident { $($variant:ident { $($field:ident),* }),* $(,)? }) => {
        impl From<$source> for $target {
            fn from(config: $source) -> Self {
                match config {
                    $($source::$variant { $($field),* } => {
                        $target::$variant { $($field),* }
                    }),*
                }
            }
        }
    };
}

impl_config_conversion!(ChannelConfigInput => DbAlertChannelConfig {
    Slack { webhook_url },
    Discord { webhook_url },
    Telegram { bot_token, chat_id },
    Email {
        smtp_host,
        smtp_port,
        smtp_username,
        smtp_password,
        from_address,
        to_address
    },
});

impl_config_conversion!(RuleConfigInput => DbAlertRuleConfig {
    ServerCpu { threshold_percent },
    ServerMemory { threshold_percent },
    ServerLoadAverage { threshold },
    AppCpu {
        app_id,
        threshold_percent
    },
    AppMemory {
        app_id,
        threshold_percent
    },
    DomainTlsExpiry {
        domain,
        days_before
    },
    DomainDnsMisconfigured { domain },
    AppHealthCheck { app_id, health_check_url },
    CronFailed { cron_job_id },
});
