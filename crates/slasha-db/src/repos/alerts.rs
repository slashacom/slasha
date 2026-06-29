use chrono::Utc;
use diesel::prelude::*;
use serde::Serialize;

use crate::{
    connection::DbPool,
    error::{DbError, DbResult},
    models::{
        alerts::{AlertChannel, AlertIncident, AlertIncidentStatus, AlertNotification, AlertRule},
        schema::{alert_channels, alert_incidents, alert_notifications, alert_rules},
    },
};

pub struct AlertChannelRepo;

impl AlertChannelRepo {
    pub async fn list(pool: &DbPool) -> DbResult<Vec<AlertChannel>> {
        let pool = pool.clone();
        tokio::task::spawn_blocking(move || {
            let mut conn = pool.get()?;
            Ok(alert_channels::table
                .order(alert_channels::created_at.desc())
                .load::<AlertChannel>(&mut conn)?)
        })
        .await?
    }

    pub async fn find_by_id(pool: &DbPool, id: &str) -> DbResult<AlertChannel> {
        let pool = pool.clone();
        let id = id.to_string();
        tokio::task::spawn_blocking(move || {
            let mut conn = pool.get()?;
            alert_channels::table
                .filter(alert_channels::id.eq(&id))
                .first::<AlertChannel>(&mut conn)
                .optional()?
                .ok_or_else(|| DbError::NotFound(format!("alert channel '{}' not found", id)))
        })
        .await?
    }

    pub async fn list_enabled_by_ids(
        pool: &DbPool,
        ids: Vec<String>,
    ) -> DbResult<Vec<AlertChannel>> {
        let pool = pool.clone();
        tokio::task::spawn_blocking(move || {
            let mut conn = pool.get()?;
            Ok(alert_channels::table
                .filter(alert_channels::id.eq_any(ids))
                .filter(alert_channels::enabled.eq(true))
                .order(alert_channels::created_at.asc())
                .load::<AlertChannel>(&mut conn)?)
        })
        .await?
    }

    pub async fn create(pool: &DbPool, channel: AlertChannel) -> DbResult<AlertChannel> {
        let pool = pool.clone();
        tokio::task::spawn_blocking(move || {
            let mut conn = pool.get()?;
            let config_json = json_string(&channel.config)?;

            diesel::insert_into(alert_channels::table)
                .values((
                    alert_channels::id.eq(&channel.id),
                    alert_channels::name.eq(&channel.name),
                    alert_channels::kind.eq(channel.kind()),
                    alert_channels::config_json.eq(config_json),
                    alert_channels::enabled.eq(channel.enabled),
                    alert_channels::created_at.eq(channel.created_at),
                    alert_channels::updated_at.eq(channel.updated_at),
                ))
                .execute(&mut conn)?;

            Ok(channel)
        })
        .await?
    }

    pub async fn update(pool: &DbPool, id: &str, channel: AlertChannel) -> DbResult<AlertChannel> {
        let pool = pool.clone();
        let id = id.to_string();
        tokio::task::spawn_blocking(move || {
            let mut conn = pool.get()?;
            let updated_at = Utc::now().naive_utc();
            let config_json = json_string(&channel.config)?;

            diesel::update(alert_channels::table.filter(alert_channels::id.eq(&id)))
                .set((
                    alert_channels::name.eq(&channel.name),
                    alert_channels::kind.eq(channel.kind()),
                    alert_channels::config_json.eq(config_json),
                    alert_channels::enabled.eq(channel.enabled),
                    alert_channels::updated_at.eq(updated_at),
                ))
                .execute(&mut conn)?;

            alert_channels::table
                .filter(alert_channels::id.eq(&id))
                .first::<AlertChannel>(&mut conn)
                .optional()?
                .ok_or_else(|| DbError::NotFound(format!("alert channel '{}' not found", id)))
        })
        .await?
    }

    pub async fn delete(pool: &DbPool, id: &str) -> DbResult<usize> {
        let pool = pool.clone();
        let id = id.to_string();
        tokio::task::spawn_blocking(move || {
            let mut conn = pool.get()?;
            Ok(
                diesel::delete(alert_channels::table.filter(alert_channels::id.eq(&id)))
                    .execute(&mut conn)?,
            )
        })
        .await?
    }
}

pub struct AlertRuleRepo;

impl AlertRuleRepo {
    pub async fn list(pool: &DbPool) -> DbResult<Vec<AlertRule>> {
        let pool = pool.clone();
        tokio::task::spawn_blocking(move || {
            let mut conn = pool.get()?;
            Ok(alert_rules::table
                .order(alert_rules::created_at.desc())
                .load::<AlertRule>(&mut conn)?)
        })
        .await?
    }

    pub async fn list_enabled(pool: &DbPool) -> DbResult<Vec<AlertRule>> {
        let pool = pool.clone();
        tokio::task::spawn_blocking(move || {
            let mut conn = pool.get()?;
            Ok(alert_rules::table
                .filter(alert_rules::enabled.eq(true))
                .order(alert_rules::created_at.asc())
                .load::<AlertRule>(&mut conn)?)
        })
        .await?
    }

    pub async fn find_by_id(pool: &DbPool, id: &str) -> DbResult<AlertRule> {
        let pool = pool.clone();
        let id = id.to_string();
        tokio::task::spawn_blocking(move || {
            let mut conn = pool.get()?;
            alert_rules::table
                .filter(alert_rules::id.eq(&id))
                .first::<AlertRule>(&mut conn)
                .optional()?
                .ok_or_else(|| DbError::NotFound(format!("alert rule '{}' not found", id)))
        })
        .await?
    }

    pub async fn create(pool: &DbPool, rule: AlertRule) -> DbResult<AlertRule> {
        let pool = pool.clone();
        tokio::task::spawn_blocking(move || {
            let mut conn = pool.get()?;
            let config_json = json_string(&rule.config)?;
            let channel_ids_json = json_string(&rule.channel_ids)?;

            diesel::insert_into(alert_rules::table)
                .values((
                    alert_rules::id.eq(&rule.id),
                    alert_rules::name.eq(&rule.name),
                    alert_rules::kind.eq(rule.kind()),
                    alert_rules::config_json.eq(config_json),
                    alert_rules::channel_ids_json.eq(channel_ids_json),
                    alert_rules::message_template.eq(&rule.message_template),
                    alert_rules::shell_command.eq(&rule.shell_command),
                    alert_rules::direct_webhook_url.eq(&rule.direct_webhook_url),
                    alert_rules::enabled.eq(rule.enabled),
                    alert_rules::cooldown_secs.eq(rule.cooldown_secs),
                    alert_rules::created_at.eq(rule.created_at),
                    alert_rules::updated_at.eq(rule.updated_at),
                ))
                .execute(&mut conn)?;

            Ok(rule)
        })
        .await?
    }

    pub async fn update(pool: &DbPool, id: &str, rule: AlertRule) -> DbResult<AlertRule> {
        let pool = pool.clone();
        let id = id.to_string();
        tokio::task::spawn_blocking(move || {
            let mut conn = pool.get()?;
            let updated_at = Utc::now().naive_utc();
            let config_json = json_string(&rule.config)?;
            let channel_ids_json = json_string(&rule.channel_ids)?;

            diesel::update(alert_rules::table.filter(alert_rules::id.eq(&id)))
                .set((
                    alert_rules::name.eq(&rule.name),
                    alert_rules::kind.eq(rule.kind()),
                    alert_rules::config_json.eq(config_json),
                    alert_rules::channel_ids_json.eq(channel_ids_json),
                    alert_rules::message_template.eq(&rule.message_template),
                    alert_rules::shell_command.eq(&rule.shell_command),
                    alert_rules::direct_webhook_url.eq(&rule.direct_webhook_url),
                    alert_rules::enabled.eq(rule.enabled),
                    alert_rules::cooldown_secs.eq(rule.cooldown_secs),
                    alert_rules::updated_at.eq(updated_at),
                ))
                .execute(&mut conn)?;

            alert_rules::table
                .filter(alert_rules::id.eq(&id))
                .first::<AlertRule>(&mut conn)
                .optional()?
                .ok_or_else(|| DbError::NotFound(format!("alert rule '{}' not found", id)))
        })
        .await?
    }

    pub async fn delete(pool: &DbPool, id: &str) -> DbResult<usize> {
        let pool = pool.clone();
        let id = id.to_string();
        tokio::task::spawn_blocking(move || {
            let mut conn = pool.get()?;
            Ok(
                diesel::delete(alert_rules::table.filter(alert_rules::id.eq(&id)))
                    .execute(&mut conn)?,
            )
        })
        .await?
    }
}

pub struct AlertIncidentRepo;

impl AlertIncidentRepo {
    pub async fn list(pool: &DbPool) -> DbResult<Vec<AlertIncident>> {
        let pool = pool.clone();
        tokio::task::spawn_blocking(move || {
            let mut conn = pool.get()?;
            Ok(alert_incidents::table
                .order(alert_incidents::opened_at.desc())
                .load::<AlertIncident>(&mut conn)?)
        })
        .await?
    }

    pub async fn find_by_id(pool: &DbPool, id: &str) -> DbResult<AlertIncident> {
        let pool = pool.clone();
        let id = id.to_string();
        tokio::task::spawn_blocking(move || {
            let mut conn = pool.get()?;
            alert_incidents::table
                .filter(alert_incidents::id.eq(&id))
                .first::<AlertIncident>(&mut conn)
                .optional()?
                .ok_or_else(|| DbError::NotFound(format!("alert incident '{}' not found", id)))
        })
        .await?
    }

    pub async fn list_open(pool: &DbPool) -> DbResult<Vec<AlertIncident>> {
        let pool = pool.clone();
        tokio::task::spawn_blocking(move || {
            let mut conn = pool.get()?;
            Ok(alert_incidents::table
                .filter(alert_incidents::status.eq(AlertIncidentStatus::Open))
                .order(alert_incidents::opened_at.desc())
                .load::<AlertIncident>(&mut conn)?)
        })
        .await?
    }

    pub async fn find_open(
        pool: &DbPool,
        rule_id: &str,
        target_key: &str,
    ) -> DbResult<Option<AlertIncident>> {
        let pool = pool.clone();
        let rule_id = rule_id.to_string();
        let target_key = target_key.to_string();
        tokio::task::spawn_blocking(move || {
            let mut conn = pool.get()?;
            Ok(alert_incidents::table
                .filter(alert_incidents::rule_id.eq(rule_id))
                .filter(alert_incidents::target_key.eq(target_key))
                .filter(alert_incidents::status.eq(AlertIncidentStatus::Open))
                .first::<AlertIncident>(&mut conn)
                .optional()?)
        })
        .await?
    }

    pub async fn create(pool: &DbPool, incident: AlertIncident) -> DbResult<AlertIncident> {
        let pool = pool.clone();
        tokio::task::spawn_blocking(move || {
            let mut conn = pool.get()?;
            diesel::insert_into(alert_incidents::table)
                .values(&incident)
                .execute(&mut conn)?;
            Ok(incident)
        })
        .await?
    }

    pub async fn touch_open(
        pool: &DbPool,
        id: &str,
        current_value: Option<f32>,
        last_notified_at: Option<chrono::NaiveDateTime>,
    ) -> DbResult<AlertIncident> {
        let pool = pool.clone();
        let id = id.to_string();
        tokio::task::spawn_blocking(move || {
            let mut conn = pool.get()?;
            diesel::update(
                alert_incidents::table
                    .filter(alert_incidents::id.eq(&id))
                    .filter(alert_incidents::status.eq(AlertIncidentStatus::Open)),
            )
            .set((
                alert_incidents::current_value.eq(current_value),
                alert_incidents::last_notified_at.eq(last_notified_at),
            ))
            .execute(&mut conn)?;

            alert_incidents::table
                .filter(alert_incidents::id.eq(&id))
                .first::<AlertIncident>(&mut conn)
                .optional()?
                .ok_or_else(|| DbError::NotFound(format!("alert incident '{}' not found", id)))
        })
        .await?
    }

    pub async fn resolve(
        pool: &DbPool,
        id: &str,
        recovery_value: Option<f32>,
    ) -> DbResult<AlertIncident> {
        let pool = pool.clone();
        let id = id.to_string();
        tokio::task::spawn_blocking(move || {
            let mut conn = pool.get()?;
            let now = Utc::now().naive_utc();
            diesel::update(
                alert_incidents::table
                    .filter(alert_incidents::id.eq(&id))
                    .filter(alert_incidents::status.eq(AlertIncidentStatus::Open)),
            )
            .set((
                alert_incidents::status.eq(AlertIncidentStatus::Resolved),
                alert_incidents::recovery_value.eq(recovery_value),
                alert_incidents::resolved_at.eq(Some(now)),
            ))
            .execute(&mut conn)?;

            alert_incidents::table
                .filter(alert_incidents::id.eq(&id))
                .first::<AlertIncident>(&mut conn)
                .optional()?
                .ok_or_else(|| DbError::NotFound(format!("alert incident '{}' not found", id)))
        })
        .await?
    }
}

pub struct AlertNotificationRepo;

impl AlertNotificationRepo {
    pub async fn list(pool: &DbPool) -> DbResult<Vec<AlertNotification>> {
        let pool = pool.clone();
        tokio::task::spawn_blocking(move || {
            let mut conn = pool.get()?;
            Ok(alert_notifications::table
                .order(alert_notifications::created_at.desc())
                .load::<AlertNotification>(&mut conn)?)
        })
        .await?
    }

    pub async fn list_for_incident(
        pool: &DbPool,
        incident_id: &str,
    ) -> DbResult<Vec<AlertNotification>> {
        let pool = pool.clone();
        let incident_id = incident_id.to_string();
        tokio::task::spawn_blocking(move || {
            let mut conn = pool.get()?;
            Ok(alert_notifications::table
                .filter(alert_notifications::incident_id.eq(incident_id))
                .order(alert_notifications::created_at.asc())
                .load::<AlertNotification>(&mut conn)?)
        })
        .await?
    }

    pub async fn create(
        pool: &DbPool,
        notification: AlertNotification,
    ) -> DbResult<AlertNotification> {
        let pool = pool.clone();
        tokio::task::spawn_blocking(move || {
            let mut conn = pool.get()?;
            diesel::insert_into(alert_notifications::table)
                .values(&notification)
                .execute(&mut conn)?;
            Ok(notification)
        })
        .await?
    }
}

fn json_string<T: Serialize>(value: &T) -> DbResult<String> {
    serde_json::to_string(value).map_err(|e| DbError::Data(e.to_string()))
}
