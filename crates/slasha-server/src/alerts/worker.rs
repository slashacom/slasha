use std::collections::{HashMap, HashSet};

use chrono::Utc;
use reqwest::Client;
use slasha_db::{
    DbPool,
    models::alerts::{
        AlertIncident, AlertIncidentStatus, AlertNotification, AlertNotificationKind, AlertRule,
        AlertRuleConfig,
    },
    repos::{
        alerts::{AlertIncidentRepo, AlertNotificationRepo, AlertRuleRepo},
        app_metrics::AppMetricsRepo,
        server_metrics::ServerMetricsRepo,
    },
};
use tokio::time::sleep;
use tracing::{error, info, warn};

use crate::{
    alerts::{delivery, evaluation::evaluate_rule},
    domain_health,
    state::Config,
};

pub struct AppSnapshot {
    pub metric: Option<slasha_db::app_metrics::AppMetrics>,
}

pub struct AlertSnapshot {
    pub server_metric: Option<slasha_db::models::server_metrics::ServerMetrics>,
    pub apps: HashMap<String, AppSnapshot>,
    pub domains: HashMap<String, domain_health::DomainHealth>,
}

pub fn spawn_alert_worker(db_pool: DbPool, config: Config) {
    tokio::spawn(async move {
        let http_client = Client::new();

        info!("alert worker started");

        loop {
            if let Err(err) = run_tick(&db_pool, &config, &http_client).await {
                error!(target: "slasha::alerts", error = ?err, "alert worker tick failed");
            }

            sleep(super::CHECK_INTERVAL).await;
        }
    });
}

async fn run_tick(db_pool: &DbPool, config: &Config, http_client: &Client) -> anyhow::Result<()> {
    let rules = AlertRuleRepo::list_enabled(db_pool).await?;
    if rules.is_empty() {
        return Ok(());
    }

    let snapshot = build_snapshot(db_pool, config, &rules).await;
    for rule in &rules {
        if let Err(err) = process_rule(db_pool, http_client, rule, &snapshot).await {
            error!(
                target: "slasha::alerts",
                rule_id = %rule.id,
                rule_name = %rule.name,
                error = ?err,
                "failed to process alert rule",
            );
        }
    }

    Ok(())
}

async fn build_snapshot(db_pool: &DbPool, config: &Config, rules: &[AlertRule]) -> AlertSnapshot {
    let mut app_ids = HashSet::new();
    let mut domains = HashSet::new();

    for rule in rules {
        match &rule.config {
            AlertRuleConfig::AppCpu { app_id, .. } | AlertRuleConfig::AppMemory { app_id, .. } => {
                app_ids.insert(app_id.clone());
            }
            AlertRuleConfig::DomainTlsExpiry { domain, .. }
            | AlertRuleConfig::DomainDnsMisconfigured { domain } => {
                domains.insert(domain.clone());
            }
            _ => {}
        }
    }

    let server_metric = match ServerMetricsRepo::find_latest(db_pool).await {
        Ok(metric) => metric,
        Err(err) => {
            warn!(target: "slasha::alerts", error = ?err, "failed to load server metrics");
            None
        }
    };

    let mut apps = HashMap::new();
    for app_id in app_ids {
        let app_metric = match AppMetricsRepo::find_latest(db_pool, &app_id).await {
            Ok(metric) => metric,
            Err(err) => {
                warn!(target: "slasha::alerts", app_id = %app_id, error = ?err, "failed to load app metrics for alert rule");
                None
            }
        };

        apps.insert(app_id, AppSnapshot { metric: app_metric });
    }

    let domains = if domains.is_empty() {
        HashMap::new()
    } else {
        let checked = domain_health::check_domains(domains.into_iter().collect(), config).await;
        checked
            .into_iter()
            .map(|health| (health.domain.clone(), health))
            .collect()
    };

    AlertSnapshot {
        server_metric,
        apps,
        domains,
    }
}

async fn process_rule(
    db_pool: &DbPool,
    http_client: &Client,
    rule: &AlertRule,
    snapshot: &AlertSnapshot,
) -> anyhow::Result<()> {
    let Some(eval) = evaluate_rule(rule, snapshot) else {
        return Ok(());
    };

    let now = Utc::now().naive_utc();
    let open_incident = AlertIncidentRepo::find_open(db_pool, &rule.id, &eval.target_key).await?;

    let (incident_id, notification_kind, should_notify, opened_at) = if eval.triggered {
        match open_incident {
            None => {
                let incident = AlertIncident {
                    id: uuid::Uuid::new_v4().to_string(),
                    rule_id: rule.id.clone(),
                    target_key: eval.target_key.clone(),
                    status: AlertIncidentStatus::Open,
                    trigger_value: eval.trigger_value,
                    current_value: eval.current_value,
                    recovery_value: None,
                    threshold_value: eval.threshold_value,
                    opened_at: now,
                    last_notified_at: Some(now),
                    resolved_at: None,
                };

                let incident = AlertIncidentRepo::create(db_pool, incident).await?;
                (incident.id, AlertNotificationKind::Triggered, true, None)
            }
            Some(incident) => {
                let incident_opened_at = incident.opened_at;
                let should_renotify = incident
                    .last_notified_at
                    .is_none_or(|last| elapsed_secs(last, now) >= i64::from(rule.cooldown_secs));
                let next_notified_at = if should_renotify {
                    Some(now)
                } else {
                    incident.last_notified_at
                };

                let incident = AlertIncidentRepo::touch_open(
                    db_pool,
                    &incident.id,
                    eval.current_value,
                    next_notified_at,
                )
                .await?;

                (
                    incident.id,
                    AlertNotificationKind::Renotified,
                    should_renotify,
                    Some(incident_opened_at),
                )
            }
        }
    } else {
        let Some(incident) = open_incident else {
            return Ok(());
        };

        let incident_opened_at = incident.opened_at;
        let incident =
            AlertIncidentRepo::resolve(db_pool, &incident.id, eval.recovery_value).await?;
        (
            incident.id,
            AlertNotificationKind::Resolved,
            true,
            Some(incident_opened_at),
        )
    };

    if !should_notify {
        return Ok(());
    }

    let message = delivery::render_alert_message(rule, &eval, notification_kind, opened_at);
    let notification = AlertNotification {
        id: uuid::Uuid::new_v4().to_string(),
        incident_id: incident_id.clone(),
        kind: notification_kind,
        message: message.clone(),
        created_at: now,
    };

    if let Err(err) = AlertNotificationRepo::create(db_pool, notification).await {
        warn!(
            target: "slasha::alerts",
            rule_id = %rule.id,
            incident_id = %incident_id,
            error = ?err,
            "failed to persist alert notification",
        );
    }

    delivery::deliver_alert(
        db_pool,
        rule,
        &eval,
        notification_kind,
        &message,
        http_client,
    )
    .await;

    Ok(())
}

fn elapsed_secs(since: chrono::NaiveDateTime, now: chrono::NaiveDateTime) -> i64 {
    (now - since).num_seconds()
}
