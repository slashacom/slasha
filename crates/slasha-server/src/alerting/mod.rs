//! Alert dispatching.
//!
//! Collectors emit a typed [`AlertEvent`] describing a metric stream sample
//! (e.g. `server.cpu` at 87%). The dispatcher matches each event against the
//! enabled [`AlertRule`]s, evaluates the rule's threshold, applies the
//! per-rule cooldown, and hands firing rules to an [`actions::Action`] for
//! delivery. Collectors know nothing about Slack, Telegram, or shell commands.

pub mod actions;

use serde::Deserialize;
use slasha_db::{DbPool, models::alert_rule::AlertRule, repos::alert_rule::AlertRuleRepo};

/// A single metric-stream sample emitted by a collector.
#[derive(Debug, Clone)]
pub struct AlertEvent {
    /// The thing the sample is about: `server`, `app:<slug>`, `domain:<host>`.
    pub target: String,
    /// The metric stream identifier, e.g. `server.cpu`, `domain.cert_days`.
    pub event: String,
    /// Human label for the resource, e.g. `CPU`.
    pub title: String,
    /// The measured value the rule threshold is compared against.
    pub value: f32,
    /// Unit suffix used when rendering, e.g. `%`, `days`.
    pub unit: String,
    /// A short human description of the current reading.
    pub detail: String,
}

/// Threshold parameters parsed from a rule's `params` JSON.
///
/// A rule fires when the sampled value is `>= gt` (when set) and/or `<= lt`
/// (when set). With neither set the event always matches — used for discrete
/// events such as `deploy.failed`.
#[derive(Debug, Default, Deserialize)]
struct Threshold {
    gt: Option<f32>,
    lt: Option<f32>,
}

/// Evaluate every enabled rule for this event and deliver the ones that fire.
///
/// Errors are logged rather than propagated — a failing alert must never take
/// down a collector loop.
pub async fn dispatch(pool: &DbPool, event: AlertEvent) {
    let rules = match AlertRuleRepo::list_enabled_for_event(pool, &event.event).await {
        Ok(rules) => rules,
        Err(err) => {
            tracing::error!(target: "slasha::alerting", error = ?err, event = %event.event, "failed to load alert rules");
            return;
        }
    };

    let now = chrono::Utc::now().naive_utc();

    for rule in rules {
        if !target_matches(&rule.target, &event.target) {
            continue;
        }

        if !threshold_matches(&rule.params, event.value) {
            continue;
        }

        if in_cooldown(&rule, now) {
            continue;
        }

        // Stamp the fire time before delivery so a slow action can't let the
        // next collector tick re-fire the same rule.
        if let Err(err) = AlertRuleRepo::mark_fired(pool, &rule.id, now).await {
            tracing::error!(target: "slasha::alerting", error = ?err, rule = %rule.id, "failed to record alert fire time");
            continue;
        }

        let message = render_message(&rule, &event);
        let pool = pool.clone();
        let event = event.clone();
        tokio::spawn(async move {
            if let Err(err) = actions::execute(&pool, &rule, &event, &message).await {
                tracing::error!(target: "slasha::alerting", error = ?err, rule = %rule.id, action = %rule.action_type, "alert action failed");
            }
        });
    }
}

/// A rule target matches when it is `any`, an exact match, or a bare scope
/// prefix (`app` matches `app:foo`, `domain` matches `domain:bar`).
fn target_matches(rule_target: &str, event_target: &str) -> bool {
    if rule_target == "any" || rule_target == event_target {
        return true;
    }

    if !rule_target.contains(':') {
        return event_target
            .split_once(':')
            .is_some_and(|(scope, _)| scope == rule_target);
    }

    false
}

fn threshold_matches(params_json: &str, value: f32) -> bool {
    let threshold: Threshold = serde_json::from_str(params_json).unwrap_or_default();

    if let Some(gt) = threshold.gt
        && value < gt
    {
        return false;
    }

    if let Some(lt) = threshold.lt
        && value > lt
    {
        return false;
    }

    true
}

fn in_cooldown(rule: &AlertRule, now: chrono::NaiveDateTime) -> bool {
    let Some(last) = rule.last_fired_at else {
        return false;
    };

    let elapsed = now.signed_duration_since(last);
    elapsed < chrono::Duration::seconds(rule.cooldown_secs as i64)
}

/// Render the alert text, honouring a per-rule custom template when present.
///
/// Templates support `{{title}}`, `{{target}}`, `{{event}}`, `{{value}}`,
/// `{{unit}}`, and `{{detail}}` placeholders.
fn render_message(rule: &AlertRule, event: &AlertEvent) -> String {
    let custom = serde_json::from_str::<serde_json::Value>(&rule.action_config)
        .ok()
        .and_then(|cfg| {
            cfg.get("message")
                .and_then(|m| m.as_str())
                .map(str::to_string)
        })
        .filter(|m| !m.trim().is_empty());

    let template = custom
        .unwrap_or_else(|| format!("🚨 {} alert on {{{{target}}}}: {{{{detail}}}}", event.title));

    template
        .replace("{{title}}", &event.title)
        .replace("{{target}}", &event.target)
        .replace("{{event}}", &event.event)
        .replace("{{value}}", &format!("{:.1}", event.value))
        .replace("{{unit}}", &event.unit)
        .replace("{{detail}}", &event.detail)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn target_matching() {
        assert!(target_matches("any", "server"));
        assert!(target_matches("server", "server"));
        assert!(target_matches("app", "app:web"));
        assert!(target_matches("app:web", "app:web"));
        assert!(!target_matches("app:web", "app:api"));
        assert!(!target_matches("server", "app:web"));
    }

    #[test]
    fn threshold_evaluation() {
        assert!(threshold_matches(r#"{"gt":80}"#, 85.0));
        assert!(!threshold_matches(r#"{"gt":80}"#, 70.0));
        assert!(threshold_matches(r#"{"lt":7}"#, 3.0));
        assert!(!threshold_matches(r#"{"lt":7}"#, 10.0));
        assert!(threshold_matches("{}", 1.0));
        assert!(threshold_matches("not json", 1.0));
    }
}
