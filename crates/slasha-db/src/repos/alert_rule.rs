use diesel::prelude::*;

use crate::{
    connection::DbPool,
    error::{DbError, DbResult},
    models::{
        alert_rule::{AlertRule, AlertRuleInput},
        schema::alert_rules,
    },
};

pub struct AlertRuleRepo;

impl AlertRuleRepo {
    pub async fn list(pool: &DbPool) -> DbResult<Vec<AlertRule>> {
        let pool = pool.clone();
        tokio::task::spawn_blocking(move || {
            let mut conn = pool.get()?;
            Ok(alert_rules::table
                .order(alert_rules::created_at.asc())
                .load::<AlertRule>(&mut conn)?)
        })
        .await?
    }

    pub async fn list_enabled_for_event(pool: &DbPool, event: &str) -> DbResult<Vec<AlertRule>> {
        let pool = pool.clone();
        let event = event.to_string();
        tokio::task::spawn_blocking(move || {
            let mut conn = pool.get()?;
            Ok(alert_rules::table
                .filter(alert_rules::enabled.eq(true))
                .filter(alert_rules::event.eq(&event))
                .order(alert_rules::created_at.asc())
                .load::<AlertRule>(&mut conn)?)
        })
        .await?
    }

    pub async fn get(pool: &DbPool, id: &str) -> DbResult<AlertRule> {
        let pool = pool.clone();
        let id = id.to_string();
        tokio::task::spawn_blocking(move || {
            let mut conn = pool.get()?;
            alert_rules::table
                .find(&id)
                .first::<AlertRule>(&mut conn)
                .map_err(|err| match err {
                    diesel::result::Error::NotFound => {
                        DbError::NotFound(format!("alert rule {id} not found"))
                    }
                    other => other.into(),
                })
        })
        .await?
    }

    pub async fn create(pool: &DbPool, input: AlertRuleInput) -> DbResult<AlertRule> {
        let pool = pool.clone();
        tokio::task::spawn_blocking(move || {
            let mut conn = pool.get()?;
            let now = chrono::Utc::now().naive_utc();
            let rule = AlertRule {
                id: uuid::Uuid::new_v4().to_string(),
                name: input.name,
                enabled: input.enabled,
                target: input.target,
                event: input.event,
                params: input.params,
                cooldown_secs: input.cooldown_secs,
                action_type: input.action_type,
                action_config: input.action_config,
                last_fired_at: None,
                created_at: now,
                updated_at: now,
            };
            diesel::insert_into(alert_rules::table)
                .values(&rule)
                .execute(&mut conn)?;
            Ok(rule)
        })
        .await?
    }

    pub async fn update(pool: &DbPool, id: &str, input: AlertRuleInput) -> DbResult<AlertRule> {
        let pool = pool.clone();
        let id = id.to_string();
        tokio::task::spawn_blocking(move || {
            let mut conn = pool.get()?;
            let updated = diesel::update(alert_rules::table.find(&id))
                .set((
                    alert_rules::name.eq(input.name),
                    alert_rules::enabled.eq(input.enabled),
                    alert_rules::target.eq(input.target),
                    alert_rules::event.eq(input.event),
                    alert_rules::params.eq(input.params),
                    alert_rules::cooldown_secs.eq(input.cooldown_secs),
                    alert_rules::action_type.eq(input.action_type),
                    alert_rules::action_config.eq(input.action_config),
                    alert_rules::updated_at.eq(chrono::Utc::now().naive_utc()),
                ))
                .returning(AlertRule::as_returning())
                .get_result(&mut conn)
                .map_err(|err| match err {
                    diesel::result::Error::NotFound => {
                        DbError::NotFound(format!("alert rule {id} not found"))
                    }
                    other => other.into(),
                })?;
            Ok(updated)
        })
        .await?
    }

    pub async fn mark_fired(
        pool: &DbPool,
        id: &str,
        fired_at: chrono::NaiveDateTime,
    ) -> DbResult<()> {
        let pool = pool.clone();
        let id = id.to_string();
        tokio::task::spawn_blocking(move || {
            let mut conn = pool.get()?;
            diesel::update(alert_rules::table.find(&id))
                .set(alert_rules::last_fired_at.eq(Some(fired_at)))
                .execute(&mut conn)?;
            Ok(())
        })
        .await?
    }

    pub async fn delete(pool: &DbPool, id: &str) -> DbResult<()> {
        let pool = pool.clone();
        let id = id.to_string();
        tokio::task::spawn_blocking(move || {
            let mut conn = pool.get()?;
            diesel::delete(alert_rules::table.find(&id)).execute(&mut conn)?;
            Ok(())
        })
        .await?
    }
}
