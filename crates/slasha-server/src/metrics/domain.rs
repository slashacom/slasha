use slasha_db::{DbPool, repos::app_domain::AppDomainRepo};

use crate::{
    alerting::{self, AlertEvent},
    domain_health::{self, DnsStatus, DomainHealth},
    metrics::DOMAIN_CHECK_INTERVAL,
    state::Config,
};

/// Periodically probes every app domain's DNS/TLS health and emits alert
/// events for expiring certificates and DNS drift. Unlike the on-demand
/// `/domains/health` endpoint, this runs on a schedule so problems are caught
/// without anyone watching the dashboard.
pub struct DomainHealthCollector {
    db_pool: DbPool,
    config: Config,
}

impl DomainHealthCollector {
    pub fn new(db_pool: DbPool, config: Config) -> Self {
        Self { db_pool, config }
    }

    pub fn spawn(self) {
        tokio::spawn(async move {
            tracing::info!("domain health collector started");

            loop {
                tokio::time::sleep(DOMAIN_CHECK_INTERVAL).await;

                if let Err(err) = self.tick().await {
                    tracing::error!(target: "slasha::metrics", error = ?err, "domain health check failed");
                }
            }
        });
    }

    async fn tick(&self) -> anyhow::Result<()> {
        let domains = AppDomainRepo::list_all(&self.db_pool).await?;
        if domains.is_empty() {
            return Ok(());
        }

        let names: Vec<String> = domains.into_iter().map(|d| d.domain).collect();
        let results = domain_health::check_domains(names, &self.config).await;

        for health in results {
            self.emit(&health).await;
        }

        Ok(())
    }

    async fn emit(&self, health: &DomainHealth) {
        let target = format!("domain:{}", health.domain);

        if let Some(days) = health.tls.days_until_expiry {
            alerting::dispatch(
                &self.db_pool,
                AlertEvent {
                    target: target.clone(),
                    event: "domain.cert_days".into(),
                    title: "TLS certificate".into(),
                    value: days as f32,
                    unit: "days".into(),
                    detail: format!("Certificate for {} expires in {} days", health.domain, days),
                },
            )
            .await;
        }

        let dns_problem = matches!(
            health.dns.status,
            DnsStatus::Mismatch | DnsStatus::Unresolved
        );

        alerting::dispatch(
            &self.db_pool,
            AlertEvent {
                target,
                event: "domain.dns_problem".into(),
                title: "DNS".into(),
                value: if dns_problem { 1.0 } else { 0.0 },
                unit: "".into(),
                detail: format!(
                    "DNS for {} resolves to {}",
                    health.domain,
                    if health.dns.resolved_ips.is_empty() {
                        "nothing".to_string()
                    } else {
                        health.dns.resolved_ips.join(", ")
                    }
                ),
            },
        )
        .await;
    }
}
