use slasha_db::{alerts::AlertRule, cron::CronRunStatus, models::alerts::AlertRuleConfig};

use super::worker::{AlertSnapshot, AppSnapshot};

#[derive(Clone)]
pub struct EvaluationResult {
    pub target_key: String,
    pub trigger_value: Option<f32>,
    pub current_value: Option<f32>,
    pub recovery_value: Option<f32>,
    pub threshold_value: Option<f32>,
    pub detail_display: String,
    pub triggered: bool,
}

pub fn evaluate_rule(rule: &AlertRule, snapshot: &AlertSnapshot) -> Option<EvaluationResult> {
    let target_key = rule.config.generate_target_key();
    let mut result = match &rule.config {
        AlertRuleConfig::ServerCpu { threshold_percent } => {
            evaluate_server_cpu(snapshot, *threshold_percent)
        }
        AlertRuleConfig::ServerMemory { threshold_percent } => {
            evaluate_server_memory(snapshot, *threshold_percent)
        }
        AlertRuleConfig::ServerLoadAverage { threshold } => {
            evaluate_server_load_average(snapshot, *threshold)
        }
        AlertRuleConfig::AppCpu {
            app_id,
            threshold_percent,
        } => evaluate_app_cpu(snapshot.apps.get(app_id)?, *threshold_percent),
        AlertRuleConfig::AppMemory {
            app_id,
            threshold_percent,
        } => evaluate_app_memory(snapshot.apps.get(app_id)?, *threshold_percent),
        AlertRuleConfig::DomainTlsExpiry {
            domain,
            days_before,
        } => evaluate_domain_tls_expiry(snapshot, domain, *days_before),
        AlertRuleConfig::DomainDnsMisconfigured { domain } => evaluate_domain_dns(snapshot, domain),
        AlertRuleConfig::AppHealthCheck { app_id, url } => {
            evaluate_app_health_check(snapshot.apps.get(app_id)?, url)
        }
        AlertRuleConfig::CronFailed { cron_job_id } => evaluate_cron_failed(snapshot, cron_job_id),
    }?;

    result.target_key = target_key;
    Some(result)
}

fn evaluate_server_cpu(snapshot: &AlertSnapshot, threshold: f32) -> Option<EvaluationResult> {
    let metric = snapshot.server_metric.as_ref()?;
    let current = metric.cpu_usage;
    Some(EvaluationResult {
        target_key: String::new(),
        trigger_value: Some(current),
        current_value: Some(current),
        recovery_value: Some(current),
        threshold_value: Some(threshold),
        detail_display: format!("CPU usage at {current:.1}%, threshold {threshold:.1}%"),
        triggered: current >= threshold,
    })
}

fn evaluate_server_memory(snapshot: &AlertSnapshot, threshold: f32) -> Option<EvaluationResult> {
    let metric = snapshot.server_metric.as_ref()?;
    let current = percent(metric.memory_used, metric.memory_total);
    Some(EvaluationResult {
        target_key: String::new(),
        trigger_value: Some(current),
        current_value: Some(current),
        recovery_value: Some(current),
        threshold_value: Some(threshold),
        detail_display: format!("Memory usage at {current:.1}%, threshold {threshold:.1}%"),
        triggered: current >= threshold,
    })
}

fn evaluate_server_load_average(
    snapshot: &AlertSnapshot,
    threshold: f32,
) -> Option<EvaluationResult> {
    let metric = snapshot.server_metric.as_ref()?;
    let current = metric.load_average;
    Some(EvaluationResult {
        target_key: String::new(),
        trigger_value: Some(current),
        current_value: Some(current),
        recovery_value: Some(current),
        threshold_value: Some(threshold),
        detail_display: format!("Load Average at {current:.2}, threshold {threshold:.2}"),
        triggered: current >= threshold,
    })
}

fn evaluate_app_cpu(snapshot: &AppSnapshot, threshold: f32) -> Option<EvaluationResult> {
    let current = snapshot.metric.as_ref()?.cpu_usage;
    Some(EvaluationResult {
        target_key: String::new(),
        trigger_value: Some(current),
        current_value: Some(current),
        recovery_value: Some(current),
        threshold_value: Some(threshold),
        detail_display: format!("App CPU at {current:.1}%, threshold {threshold:.1}%"),
        triggered: current >= threshold,
    })
}

fn evaluate_app_memory(snapshot: &AppSnapshot, threshold: f32) -> Option<EvaluationResult> {
    let metric = snapshot.metric.as_ref()?;
    let current = percent(metric.memory_used, metric.memory_limit);
    Some(EvaluationResult {
        target_key: String::new(),
        trigger_value: Some(current),
        current_value: Some(current),
        recovery_value: Some(current),
        threshold_value: Some(threshold),
        detail_display: format!("App Memory at {current:.1}%, threshold {threshold:.1}%"),
        triggered: current >= threshold,
    })
}

fn evaluate_domain_tls_expiry(
    snapshot: &AlertSnapshot,
    domain: &str,
    days_before: i32,
) -> Option<EvaluationResult> {
    let health = snapshot.domains.get(domain)?;
    let days_until = health.tls.days_until_expiry?;
    let triggered = matches!(
        health.tls.status,
        crate::domain_health::TlsStatus::Active | crate::domain_health::TlsStatus::Expired
    ) && days_until <= i64::from(days_before);

    let days_str = if days_until >= 0 {
        format!("{days_until} days remaining")
    } else {
        format!("expired {} days ago", days_until.abs())
    };
    let detail_display = match &health.tls.expires_at {
        Some(expires_at) => format!("{days_str}, expires at {expires_at}"),
        None => days_str,
    };

    Some(EvaluationResult {
        target_key: String::new(),
        trigger_value: Some(days_until as f32),
        current_value: Some(days_until as f32),
        recovery_value: Some(days_until as f32),
        threshold_value: Some(days_before as f32),
        detail_display,
        triggered,
    })
}

fn evaluate_domain_dns(snapshot: &AlertSnapshot, domain: &str) -> Option<EvaluationResult> {
    let health = snapshot.domains.get(domain)?;
    let triggered = matches!(
        health.dns.status,
        crate::domain_health::DnsStatus::Mismatch | crate::domain_health::DnsStatus::Unresolved
    );

    let resolved = if health.dns.resolved_ips.is_empty() {
        "none".to_string()
    } else {
        health.dns.resolved_ips.join(", ")
    };
    let expected = if health.dns.expected_ips.is_empty() {
        "unknown".to_string()
    } else {
        health.dns.expected_ips.join(", ")
    };

    let detail_display = if triggered {
        format!("DNS misconfigured or unresolved. Resolves to {resolved}, but expected {expected}")
    } else if let Some(proxy) = health.dns.proxy {
        format!("DNS proxied through {proxy}. Resolves to {resolved}")
    } else {
        format!("DNS correctly configured. Resolves to expected IPs: {expected}")
    };

    Some(EvaluationResult {
        target_key: String::new(),
        trigger_value: None,
        current_value: None,
        recovery_value: None,
        threshold_value: None,
        detail_display,
        triggered,
    })
}

fn evaluate_app_health_check(snapshot: &AppSnapshot, url: &str) -> Option<EvaluationResult> {
    let healthy = snapshot.health_check?;
    let detail_display = if healthy {
        format!("Responded with 2xx success status: {url}")
    } else {
        format!("Failed to respond with 2xx status (error or timeout): {url}")
    };
    Some(EvaluationResult {
        target_key: String::new(),
        trigger_value: None,
        current_value: None,
        recovery_value: None,
        threshold_value: None,
        detail_display,
        triggered: !healthy,
    })
}

fn evaluate_cron_failed(snapshot: &AlertSnapshot, cron_job_id: &str) -> Option<EvaluationResult> {
    let latest = snapshot.crons.get(cron_job_id)?;
    let (triggered, detail_display) = match latest {
        Some(run) => {
            let failed = matches!(run.status, CronRunStatus::Failed | CronRunStatus::TimedOut);
            let detail = match run.exit_code {
                Some(code) => format!("Last run {} (exit code {})", run.status, code),
                None => format!("Last run {}", run.status),
            };
            (failed, detail)
        }
        None => (false, "No completed runs yet".to_string()),
    };

    Some(EvaluationResult {
        target_key: String::new(),
        trigger_value: None,
        current_value: None,
        recovery_value: None,
        threshold_value: None,
        detail_display,
        triggered,
    })
}

fn percent(used: i32, total: i32) -> f32 {
    if total == 0 {
        0.0
    } else {
        used as f32 / total as f32 * 100.0
    }
}
