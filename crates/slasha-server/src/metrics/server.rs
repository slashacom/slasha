use std::{path::Path, time::Instant};

use slasha_db::{
    DbPool,
    models::{server_metrics::ServerMetrics, server_settings::ServerSettings},
    repos::{server_metrics::ServerMetricsRepo, server_settings::ServerSettingsRepo},
};
use sysinfo::{Disks, Networks, System};

use crate::metrics::{COLLECT_INTERVAL, utils::bytes_to_mib};

#[derive(Copy, Clone)]
enum AlertKind {
    Cpu,
    Memory,
    Disk,
}

struct Alert {
    kind: AlertKind,
    name: &'static str,
    emoji: &'static str,
    current: f32,
    limit: f32,
}

#[derive(Default)]
struct AlertState {
    cpu: Option<Instant>,
    memory: Option<Instant>,
    disk: Option<Instant>,
}

pub struct ServerMetricsCollector {
    db_pool: DbPool,
    system: System,
    networks: Networks,
    disks: Disks,
    last_refresh: Instant,
    alert_state: AlertState,
    http_client: reqwest::Client,
}

impl ServerMetricsCollector {
    pub fn new(db_pool: DbPool) -> Self {
        let mut system = System::new();
        system.refresh_cpu_usage();
        system.refresh_memory();

        Self {
            db_pool,
            system,
            networks: Networks::new_with_refreshed_list(),
            disks: Disks::new_with_refreshed_list(),
            last_refresh: Instant::now(),
            alert_state: AlertState::default(),
            http_client: reqwest::Client::new(),
        }
    }

    pub fn spawn(mut self) {
        tokio::spawn(async move {
            tracing::info!("server metrics collector started");

            loop {
                tokio::time::sleep(COLLECT_INTERVAL).await;

                let metric = self.sample();

                if let Err(err) = ServerMetricsRepo::insert(&self.db_pool, metric.clone()).await {
                    tracing::error!(target: "slasha::metrics", error = ?err, "failed to persist server metrics");
                }

                if let Ok(settings) = ServerSettingsRepo::get(&self.db_pool).await
                    && let Some(ref webhook_url) = settings.slack_webhook_url
                {
                    self.check_alerts(&metric, &settings, webhook_url).await;
                }

                let cutoff = chrono::Utc::now().naive_utc() - chrono::Duration::days(7);
                if let Err(err) = ServerMetricsRepo::prune_older_than(&self.db_pool, cutoff).await {
                    tracing::error!(target: "slasha::metrics", error = ?err, "failed to prune server metrics");
                }
            }
        });
    }

    async fn check_alerts(
        &mut self,
        metric: &ServerMetrics,
        settings: &ServerSettings,
        webhook_url: &str,
    ) {
        let cooldown = std::time::Duration::from_secs(15 * 60); // 15 mins cooldown
        let now = Instant::now();

        let mut alerts = Vec::new();

        if let Some(limit) = settings.cpu_limit_percent
            && metric.cpu_usage >= limit
        {
            alerts.push(Alert {
                kind: AlertKind::Cpu,
                name: "CPU",
                emoji: "🖥️",
                current: metric.cpu_usage,
                limit,
            });
        }

        if let Some(limit) = settings.memory_limit_percent {
            let pct = percent(metric.memory_used, metric.memory_total);
            if pct >= limit {
                alerts.push(Alert {
                    kind: AlertKind::Memory,
                    name: "Memory",
                    emoji: "🧠",
                    current: pct,
                    limit,
                });
            }
        }

        if let Some(limit) = settings.disk_limit_percent {
            let pct = percent(metric.disk_used, metric.disk_total);
            if pct >= limit {
                alerts.push(Alert {
                    kind: AlertKind::Disk,
                    name: "Disk",
                    emoji: "💾",
                    current: pct,
                    limit,
                });
            }
        }

        if alerts.is_empty() {
            return;
        }

        let should_send = alerts.iter().any(|alert| {
            self.last_alert(alert.kind)
                .is_none_or(|t| now.duration_since(t) >= cooldown)
        });

        if !should_send {
            return;
        }

        for alert in &alerts {
            self.set_last_alert(alert.kind, now);
        }

        let payload = serde_json::json!({
            "text": build_alert_message(&alerts)
        });

        let webhook_url = webhook_url.to_string();
        let client = self.http_client.clone();
        tokio::spawn(async move {
            if let Err(err) = client.post(&webhook_url).json(&payload).send().await {
                tracing::error!(target: "slasha::metrics", error = ?err, "failed to send slack alert");
            }
        });
    }

    fn last_alert(&self, kind: AlertKind) -> Option<Instant> {
        match kind {
            AlertKind::Cpu => self.alert_state.cpu,
            AlertKind::Memory => self.alert_state.memory,
            AlertKind::Disk => self.alert_state.disk,
        }
    }

    fn set_last_alert(&mut self, kind: AlertKind, now: Instant) {
        match kind {
            AlertKind::Cpu => self.alert_state.cpu = Some(now),
            AlertKind::Memory => self.alert_state.memory = Some(now),
            AlertKind::Disk => self.alert_state.disk = Some(now),
        }
    }

    fn sample(&mut self) -> ServerMetrics {
        self.system.refresh_cpu_usage();
        self.system.refresh_memory();
        self.networks.refresh(true);
        self.disks.refresh(true);

        let dt = self.last_refresh.elapsed().as_secs_f64().max(0.1);
        self.last_refresh = Instant::now();

        let (rx_bytes, tx_bytes) = self.networks.values().fold((0u64, 0u64), |(rx, tx), data| {
            (rx + data.received(), tx + data.transmitted())
        });

        let (disk_used, disk_total) = root_disk_usage(&self.disks);
        let load = System::load_average();

        ServerMetrics {
            id: uuid::Uuid::new_v4().to_string(),
            cpu_usage: self.system.global_cpu_usage(),
            memory_used: bytes_to_mib(self.system.used_memory()),
            memory_total: bytes_to_mib(self.system.total_memory()),
            swap_used: bytes_to_mib(self.system.used_swap()),
            swap_total: bytes_to_mib(self.system.total_swap()),
            disk_used: bytes_to_mib(disk_used),
            disk_total: bytes_to_mib(disk_total),
            network_rx_bps: (rx_bytes as f64 / dt) as f32,
            network_tx_bps: (tx_bytes as f64 / dt) as f32,
            load_average: load.one as f32,
            created_at: chrono::Utc::now().naive_utc(),
        }
    }
}

fn root_disk_usage(disks: &Disks) -> (u64, u64) {
    let root = Path::new("/");
    let disk = disks
        .list()
        .iter()
        .find(|d| d.mount_point() == root)
        .or_else(|| disks.list().iter().max_by_key(|d| d.total_space()));

    match disk {
        Some(d) => (
            d.total_space().saturating_sub(d.available_space()),
            d.total_space(),
        ),
        None => (0, 0),
    }
}

fn percent(used: i32, total: i32) -> f32 {
    if total == 0 {
        0.0
    } else {
        used as f32 / total as f32 * 100.0
    }
}

fn build_alert_message(alerts: &[Alert]) -> String {
    let mut out = String::from("🚨 *Server Resource Alert*\n\n");

    for alert in alerts {
        out.push_str(&format!(
            "• {} *{}*\n  Current: *{:.1}%*\n  Limit:   {:.1}%\n\n",
            alert.emoji, alert.name, alert.current, alert.limit,
        ));
    }

    out.push_str(&format!(
        "_Time: {} UTC_",
        chrono::Utc::now().format("%Y-%m-%d %H:%M:%S")
    ));

    out
}
