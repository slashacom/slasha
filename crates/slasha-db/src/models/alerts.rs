use std::str::FromStr;

use chrono::NaiveDateTime;
use diesel::{
    backend::Backend,
    deserialize::{self, FromSql},
    expression::AsExpression,
    prelude::*,
    serialize::{self, IsNull, Output, ToSql},
    sql_types::{Bool, Integer, Nullable, Text, Timestamp},
    sqlite::Sqlite,
};
use serde::{Deserialize, Serialize};
use strum_macros::{Display, EnumString};
use ts_rs::TS;

use crate::models::alerts::deserialize::FromSqlRow;

#[derive(
    Debug,
    PartialEq,
    Eq,
    FromSqlRow,
    AsExpression,
    Display,
    Copy,
    Clone,
    EnumString,
    Serialize,
    Deserialize,
    TS,
)]
#[strum(serialize_all = "snake_case")]
#[serde(rename_all = "snake_case")]
#[diesel(sql_type = diesel::sql_types::Text)]
#[ts(export, export_to = "./alerts.ts")]
pub enum AlertIncidentStatus {
    Open,
    Resolved,
}

impl ToSql<Text, Sqlite> for AlertIncidentStatus
where
    str: ToSql<Text, Sqlite>,
{
    fn to_sql<'b>(&'b self, out: &mut Output<'b, '_, Sqlite>) -> serialize::Result {
        out.set_value(self.to_string());
        Ok(IsNull::No)
    }
}

impl FromSql<Text, Sqlite> for AlertIncidentStatus {
    fn from_sql(bytes: <Sqlite as Backend>::RawValue<'_>) -> deserialize::Result<Self> {
        let value = <String as FromSql<Text, Sqlite>>::from_sql(bytes)?;
        AlertIncidentStatus::from_str(&value).map_err(|err| Box::new(err) as _)
    }
}

#[derive(
    Debug,
    PartialEq,
    Eq,
    FromSqlRow,
    AsExpression,
    Display,
    Copy,
    Clone,
    EnumString,
    Serialize,
    Deserialize,
    TS,
)]
#[strum(serialize_all = "snake_case")]
#[serde(rename_all = "snake_case")]
#[diesel(sql_type = diesel::sql_types::Text)]
#[ts(export, export_to = "./alerts.ts")]
pub enum AlertNotificationKind {
    Triggered,
    Renotified,
    Resolved,
}

impl ToSql<Text, Sqlite> for AlertNotificationKind
where
    str: ToSql<Text, Sqlite>,
{
    fn to_sql<'b>(&'b self, out: &mut Output<'b, '_, Sqlite>) -> serialize::Result {
        out.set_value(self.to_string());
        Ok(IsNull::No)
    }
}

impl FromSql<Text, Sqlite> for AlertNotificationKind {
    fn from_sql(bytes: <Sqlite as Backend>::RawValue<'_>) -> deserialize::Result<Self> {
        let value = <String as FromSql<Text, Sqlite>>::from_sql(bytes)?;
        AlertNotificationKind::from_str(&value).map_err(|err| Box::new(err) as _)
    }
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, TS)]
#[serde(tag = "kind", rename_all = "snake_case")]
#[ts(export, export_to = "./alerts.ts")]
pub enum AlertChannelConfig {
    Slack {
        webhook_url: String,
    },
    Discord {
        webhook_url: String,
    },
    Telegram {
        bot_token: String,
        chat_id: String,
    },
    Email {
        smtp_host: String,
        smtp_port: u16,
        smtp_username: String,
        smtp_password: String,
        from_address: String,
        to_address: String,
    },
}

impl AlertChannelConfig {
    pub fn kind(&self) -> &'static str {
        match self {
            AlertChannelConfig::Slack { .. } => "slack",
            AlertChannelConfig::Discord { .. } => "discord",
            AlertChannelConfig::Telegram { .. } => "telegram",
            AlertChannelConfig::Email { .. } => "email",
        }
    }
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, TS)]
#[serde(tag = "kind", rename_all = "snake_case")]
#[ts(export, export_to = "./alerts.ts")]
pub enum AlertRuleConfig {
    NodeCpu {
        node_id: String,
        threshold_percent: f64,
    },
    NodeMemory {
        node_id: String,
        threshold_percent: f64,
    },
    NodeLoadAverage {
        node_id: String,
        threshold: f64,
    },
    AppCpu {
        app_id: String,
        threshold_percent: f64,
    },
    AppMemory {
        app_id: String,
        threshold_percent: f64,
    },
    DomainTlsExpiry {
        domain: String,
        days_before: i32,
    },
    DomainDnsMisconfigured {
        domain: String,
    },
    AppHealthCheck {
        app_id: String,
        health_check_url: String,
    },
    CronFailed {
        cron_job_id: String,
    },
}

impl AlertRuleConfig {
    pub fn kind(&self) -> &'static str {
        match self {
            AlertRuleConfig::NodeCpu { .. } => "node_cpu",
            AlertRuleConfig::NodeMemory { .. } => "node_memory",
            AlertRuleConfig::NodeLoadAverage { .. } => "node_load_average",
            AlertRuleConfig::AppCpu { .. } => "app_cpu",
            AlertRuleConfig::AppMemory { .. } => "app_memory",
            AlertRuleConfig::DomainTlsExpiry { .. } => "domain_tls_expiry",
            AlertRuleConfig::DomainDnsMisconfigured { .. } => "domain_dns_misconfigured",
            AlertRuleConfig::AppHealthCheck { .. } => "app_health_check",
            AlertRuleConfig::CronFailed { .. } => "cron_failed",
        }
    }

    pub fn generate_target_key(&self) -> String {
        let kind = self.kind();
        match self {
            AlertRuleConfig::NodeCpu { node_id, .. }
            | AlertRuleConfig::NodeMemory { node_id, .. }
            | AlertRuleConfig::NodeLoadAverage { node_id, .. } => {
                format!("{kind}:{node_id}")
            }
            AlertRuleConfig::AppCpu { app_id, .. }
            | AlertRuleConfig::AppMemory { app_id, .. }
            | AlertRuleConfig::AppHealthCheck { app_id, .. } => {
                format!("{kind}:{app_id}")
            }
            AlertRuleConfig::DomainTlsExpiry { domain, .. }
            | AlertRuleConfig::DomainDnsMisconfigured { domain, .. } => {
                format!("{kind}:{domain}")
            }
            AlertRuleConfig::CronFailed { cron_job_id } => {
                format!("{kind}:{cron_job_id}")
            }
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, TS)]
#[ts(export, export_to = "./alerts.ts")]
pub struct AlertChannel {
    pub id: String,
    pub name: String,
    pub config: AlertChannelConfig,
    pub enabled: bool,
    pub created_at: NaiveDateTime,
    pub updated_at: NaiveDateTime,
}

pub struct NewAlertChannel {
    pub name: String,
    pub config: AlertChannelConfig,
    pub enabled: bool,
}

pub struct AlertChannelChangeset {
    pub name: String,
    pub config: AlertChannelConfig,
    pub enabled: bool,
}

impl AlertChannel {
    pub fn kind(&self) -> &'static str {
        self.config.kind()
    }
}

impl Queryable<(Text, Text, Text, Text, Bool, Timestamp, Timestamp), Sqlite> for AlertChannel {
    type Row = (
        String,
        String,
        String,
        String,
        bool,
        NaiveDateTime,
        NaiveDateTime,
    );

    fn build(row: Self::Row) -> deserialize::Result<Self> {
        let (id, name, kind, config_json, enabled, created_at, updated_at) = row;
        let config: AlertChannelConfig = serde_json::from_str(&config_json)?;

        if config.kind() != kind {
            return Err(format!(
                "alert channel '{}' has kind '{}' but config kind '{}'",
                id,
                kind,
                config.kind()
            )
            .into());
        }

        Ok(Self {
            id,
            name,
            config,
            enabled,
            created_at,
            updated_at,
        })
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, TS)]
#[ts(export, export_to = "./alerts.ts")]
pub struct AlertRule {
    pub id: String,
    pub name: String,
    pub config: AlertRuleConfig,
    pub channel_ids: Vec<String>,
    pub direct_webhook_url: Option<String>,
    pub message_template: Option<String>,
    pub shell_command: Option<String>,
    pub enabled: bool,
    pub cooldown_secs: i32,
    pub created_at: NaiveDateTime,
    pub updated_at: NaiveDateTime,
}

pub struct NewAlertRule {
    pub name: String,
    pub config: AlertRuleConfig,
    pub channel_ids: Vec<String>,
    pub direct_webhook_url: Option<String>,
    pub message_template: Option<String>,
    pub shell_command: Option<String>,
    pub enabled: bool,
    pub cooldown_secs: i32,
}

pub struct AlertRuleChangeset {
    pub name: String,
    pub config: AlertRuleConfig,
    pub channel_ids: Vec<String>,
    pub direct_webhook_url: Option<String>,
    pub message_template: Option<String>,
    pub shell_command: Option<String>,
    pub enabled: bool,
    pub cooldown_secs: i32,
}

impl AlertRule {
    pub fn kind(&self) -> &'static str {
        self.config.kind()
    }
}

impl
    Queryable<
        (
            Text,
            Text,
            Text,
            Text,
            Nullable<Text>,
            Nullable<Text>,
            Nullable<Text>,
            Bool,
            Integer,
            Timestamp,
            Timestamp,
        ),
        Sqlite,
    > for AlertRule
{
    type Row = (
        String,
        String,
        String,
        String,
        Option<String>,
        Option<String>,
        Option<String>,
        bool,
        i32,
        NaiveDateTime,
        NaiveDateTime,
    );

    fn build(row: Self::Row) -> deserialize::Result<Self> {
        let (
            id,
            name,
            config_json,
            channel_ids_json,
            direct_webhook_url,
            message_template,
            shell_command,
            enabled,
            cooldown_secs,
            created_at,
            updated_at,
        ) = row;

        let config: AlertRuleConfig = serde_json::from_str(&config_json)?;
        let channel_ids: Vec<String> = serde_json::from_str(&channel_ids_json)?;

        Ok(Self {
            id,
            name,
            config,
            channel_ids,
            direct_webhook_url,
            message_template,
            shell_command,
            enabled,
            cooldown_secs,
            created_at,
            updated_at,
        })
    }
}

#[derive(
    Queryable, Selectable, Insertable, AsChangeset, Debug, Clone, Serialize, Deserialize, TS,
)]
#[diesel(table_name = crate::models::schema::alert_incidents)]
#[ts(export, export_to = "./alerts.ts")]
pub struct AlertIncident {
    pub id: String,
    pub rule_id: String,
    pub target_key: String,
    pub status: AlertIncidentStatus,
    pub trigger_value: Option<f64>,
    pub current_value: Option<f64>,
    pub recovery_value: Option<f64>,
    pub threshold_value: Option<f64>,
    pub opened_at: NaiveDateTime,
    pub last_notified_at: Option<NaiveDateTime>,
    pub resolved_at: Option<NaiveDateTime>,
}

#[derive(Queryable, Selectable, Insertable, Debug, Clone, Serialize, Deserialize, TS)]
#[diesel(table_name = crate::models::schema::alert_notifications)]
#[ts(export, export_to = "./alerts.ts")]
pub struct AlertNotification {
    pub id: String,
    pub incident_id: String,
    pub kind: AlertNotificationKind,
    pub message: String,
    pub created_at: NaiveDateTime,
}
