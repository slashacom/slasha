use std::collections::{HashMap, HashSet};

use chrono::Utc;
use reqwest::Client;
use slasha_db::{
    DbPool, DuckdbPool,
    app_metrics::AppMetrics,
    cron::CronRun,
    models::alerts::{
        AlertIncident, AlertIncidentStatus, AlertNotification, AlertNotificationKind, AlertRule,
        AlertRuleConfig,
    },
    repos::{
        alerts::{AlertIncidentRepo, AlertNotificationRepo, AlertRuleRepo},
        app_metrics::AppMetricsRepo,
        cron::CronRunRepo,
        server_metrics::ServerMetricsRepo,
    },
    server_metrics::ServerMetrics,
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
    pub health_check: Option<bool>,
}

pub struct AlertSnapshot {
    pub server_metric: Option<ServerMetrics>,
    pub apps: HashMap<String, AppSnapshot>,
    pub domains: HashMap<String, domain_health::DomainHealth>,
    pub crons: HashMap<String, Option<CronRun>>,
}

pub fn spawn_alert_worker(db_pool: DbPool, duckdb_pool: DuckdbPool, config: Config) {
    tokio::spawn(async move {
        let http_client = Client::new();

        info!("alert worker started");

        loop {
            if let Err(err) = run_tick(&db_pool, &duckdb_pool, &config, &http_client).await {
                error!(target: "slasha::alerts", error = ?err, "alert worker tick failed");
            }

            sleep(super::CHECK_INTERVAL).await;
        }
    });
}

async fn run_tick(
    db_pool: &DbPool,
    duckdb_pool: &DuckdbPool,
    config: &Config,
    http_client: &Client,
) -> anyhow::Result<()> {
    let rules = AlertRuleRepo::list_enabled(db_pool).await?;
    if rules.is_empty() {
        return Ok(());
    }

    let snapshot = build_snapshot(db_pool, duckdb_pool, config, &rules, http_client).await;
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

async fn build_snapshot(
    db_pool: &DbPool,
    duckdb_pool: &DuckdbPool,
    config: &Config,
    rules: &[AlertRule],
    http_client: &Client,
) -> AlertSnapshot {
    let mut metric_app_ids = HashSet::new();
    let mut domains_to_check = HashSet::new();
    let mut health_check_urls = HashMap::new();
    let mut cron_job_ids = HashSet::new();

    for rule in rules {
        match &rule.config {
            AlertRuleConfig::AppCpu { app_id, .. } | AlertRuleConfig::AppMemory { app_id, .. } => {
                metric_app_ids.insert(app_id.clone());
            }

            AlertRuleConfig::AppHealthCheck {
                app_id,
                health_check_url,
            } => {
                health_check_urls.insert(app_id.clone(), health_check_url.clone());
            }

            AlertRuleConfig::DomainTlsExpiry { domain, .. }
            | AlertRuleConfig::DomainDnsMisconfigured { domain } => {
                domains_to_check.insert(domain.clone());
            }

            AlertRuleConfig::CronFailed { cron_job_id } => {
                cron_job_ids.insert(cron_job_id.clone());
            }

            _ => {}
        }
    }

    let server_metric = get_server_metric(duckdb_pool).await;

    let mut app_ids = metric_app_ids.clone();
    app_ids.extend(health_check_urls.keys().cloned());

    let mut apps = HashMap::new();

    for app_id in app_ids {
        let metric = if metric_app_ids.contains(&app_id) {
            get_app_metric(duckdb_pool, &app_id).await
        } else {
            None
        };

        let health_check = if let Some(url) = health_check_urls.get(&app_id) {
            Some(probe_health_check(http_client, url).await)
        } else {
            None
        };

        apps.insert(
            app_id,
            AppSnapshot {
                metric,
                health_check,
            },
        );
    }

    let domains = domain_health::check_domains(domains_to_check.into_iter().collect(), config)
        .await
        .into_iter()
        .map(|health| (health.domain.clone(), health))
        .collect();

    let mut crons = HashMap::new();
    for cron_job_id in cron_job_ids {
        let latest = get_cron_outcome(db_pool, &cron_job_id).await;
        crons.insert(cron_job_id, latest);
    }

    AlertSnapshot {
        server_metric,
        apps,
        domains,
        crons,
    }
}

async fn get_server_metric(duckdb_pool: &DuckdbPool) -> Option<ServerMetrics> {
    ServerMetricsRepo::get_latest(duckdb_pool, "local")
        .await
        .unwrap_or_else(|err| {
            warn!(target: "slasha::alerts", error = ?err, "failed to load server metrics");
            None
        })
}

async fn get_app_metric(duckdb_pool: &DuckdbPool, app_id: &str) -> Option<AppMetrics> {
    AppMetricsRepo::get_latest(duckdb_pool, app_id)
        .await
        .unwrap_or_else(|err| {
            warn!(target: "slasha::alerts", app_id = %app_id, error = ?err, "failed to load app metrics for alert rule");
            None
        })
}

async fn get_cron_outcome(db_pool: &DbPool, cron_job_id: &str) -> Option<CronRun> {
    CronRunRepo::latest_outcome_for_job(db_pool, cron_job_id)
        .await
        .unwrap_or_else(|err| {
            warn!(target: "slasha::alerts", cron_job_id = %cron_job_id, error = ?err, "failed to load cron run for alert rule");
            None
        })
}

async fn probe_health_check(http_client: &Client, url: &str) -> bool {
    let result = tokio::time::timeout(
        std::time::Duration::from_secs(10),
        http_client.get(url).send(),
    )
    .await;

    match result {
        Ok(Ok(response)) => response.status().is_success(),
        _ => false,
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
