use std::{
    collections::{HashMap, HashSet},
    time::Duration,
};

use bollard::{
    Docker,
    models::{ContainerBlkioStats, ContainerCpuStats, ContainerNetworkStats},
    query_parameters::{ListContainersOptionsBuilder, StatsOptionsBuilder},
};
use futures_util::StreamExt;
use slasha_db::{DbPool, models::app_metrics::AppMetrics, repos::app_metrics::AppMetricsRepo};
use tokio::time::sleep;

struct PrevCounters {
    rx_bytes: u64,
    tx_bytes: u64,
    disk_read_bytes: u64,
    disk_write_bytes: u64,
    timestamp_secs: f64,
}

#[derive(Default)]
struct AppAggregate {
    cpu_percent: f64,
    memory_used_bytes: u64,
    memory_limit_bytes: u64,
    net_rx_bps: f64,
    net_tx_bps: f64,
    disk_read_bps: f64,
    disk_write_bps: f64,
}

pub fn spawn_metrics_collector(db_pool: DbPool, docker: Docker) {
    tokio::spawn(async move {
        let mut prev: HashMap<String, PrevCounters> = HashMap::new();
        tracing::info!("app metrics collector started");

        loop {
            sleep(Duration::from_secs(10)).await;

            if let Err(err) = tick(&db_pool, &docker, &mut prev).await {
                tracing::error!(error = ?err, "metrics collection failed");
            }
        }
    });
}

async fn tick(
    db_pool: &DbPool,
    docker: &Docker,
    prev: &mut HashMap<String, PrevCounters>,
) -> anyhow::Result<()> {
    let mut filters = HashMap::new();
    filters.insert("label".to_string(), vec!["slasha.managed=true".to_string()]);
    filters.insert("status".to_string(), vec!["running".to_string()]);

    let containers = docker
        .list_containers(Some(
            ListContainersOptionsBuilder::new()
                .all(true)
                .filters(&filters)
                .build(),
        ))
        .await?;

    if containers.is_empty() {
        prev.clear();
        return Ok(());
    }

    let stats_opts = StatsOptionsBuilder::default()
        .stream(false)
        .one_shot(true)
        .build();

    let fetch_futures = containers.iter().filter_map(|c| {
        let container_id = c.id.as_deref()?;
        let app_id = c
            .labels
            .as_ref()
            .and_then(|l| l.get("slasha.app_id"))
            .cloned()?;

        let docker = docker.clone();
        let cid = container_id.to_string();
        let opts = stats_opts.clone();

        Some(async move {
            let snapshot = docker.stats(&cid, Some(opts)).next().await;
            match snapshot {
                Some(Ok(s)) => Some((cid, app_id, s)),
                Some(Err(err)) => {
                    tracing::warn!(container = %cid, error = ?err, "stats fetch failed");
                    None
                }
                None => None,
            }
        })
    });

    let snapshots: Vec<_> = futures_util::future::join_all(fetch_futures)
        .await
        .into_iter()
        .flatten()
        .collect();

    let active_ids: HashSet<&str> = snapshots.iter().map(|(cid, _, _)| cid.as_str()).collect();

    let mut app_aggregates: HashMap<String, AppAggregate> = HashMap::new();

    for (container_id, app_id, stats) in &snapshots {
        let cpu_percent =
            compute_cpu_percent(stats.cpu_stats.as_ref(), stats.precpu_stats.as_ref());

        let (mem_used, mem_limit) = stats
            .memory_stats
            .as_ref()
            .map(|m| (m.usage.unwrap_or(0), m.limit.unwrap_or(0)))
            .unwrap_or((0, 0));

        let (net_rx, net_tx) = sum_network_bytes(stats.networks.as_ref());
        let (disk_read, disk_write) = sum_blkio_bytes(stats.blkio_stats.as_ref());

        let now_secs = stats
            .read
            .as_ref()
            .map(|t| t.timestamp() as f64)
            .unwrap_or_else(|| {
                std::time::SystemTime::now()
                    .duration_since(std::time::UNIX_EPOCH)
                    .unwrap_or_default()
                    .as_secs_f64()
            });

        let (net_rx_bps, net_tx_bps, disk_read_bps, disk_write_bps) =
            if let Some(p) = prev.get(container_id.as_str()) {
                let dt = (now_secs - p.timestamp_secs).max(0.1);
                (
                    net_rx.saturating_sub(p.rx_bytes) as f64 / dt,
                    net_tx.saturating_sub(p.tx_bytes) as f64 / dt,
                    disk_read.saturating_sub(p.disk_read_bytes) as f64 / dt,
                    disk_write.saturating_sub(p.disk_write_bytes) as f64 / dt,
                )
            } else {
                (0.0, 0.0, 0.0, 0.0)
            };

        prev.insert(
            container_id.clone(),
            PrevCounters {
                rx_bytes: net_rx,
                tx_bytes: net_tx,
                disk_read_bytes: disk_read,
                disk_write_bytes: disk_write,
                timestamp_secs: now_secs,
            },
        );

        let agg = app_aggregates.entry(app_id.clone()).or_default();
        agg.cpu_percent += cpu_percent;
        agg.memory_used_bytes += mem_used;
        agg.memory_limit_bytes = agg.memory_limit_bytes.max(mem_limit);
        agg.net_rx_bps += net_rx_bps;
        agg.net_tx_bps += net_tx_bps;
        agg.disk_read_bps += disk_read_bps;
        agg.disk_write_bps += disk_write_bps;
    }

    prev.retain(|cid, _| active_ids.contains(cid.as_str()));

    let now = chrono::Utc::now().naive_utc();

    for (app_id, agg) in app_aggregates {
        let metric = AppMetrics {
            id: uuid::Uuid::new_v4().to_string(),
            app_id: app_id.clone(),
            cpu_usage: agg.cpu_percent as f32,
            memory_used: bytes_to_mib(agg.memory_used_bytes),
            memory_limit: bytes_to_mib(agg.memory_limit_bytes),
            network_rx_bps: agg.net_rx_bps as f32,
            network_tx_bps: agg.net_tx_bps as f32,
            disk_read_bps: agg.disk_read_bps as f32,
            disk_write_bps: agg.disk_write_bps as f32,
            created_at: now,
        };

        if let Err(err) = AppMetricsRepo::insert(db_pool, metric).await {
            tracing::error!(
                target: "slasha::metrics",
                app_id = %app_id,
                error = ?err,
                "failed to persist app metrics"
            );
        }
    }

    let cutoff = now - chrono::Duration::days(7);
    if let Err(err) = AppMetricsRepo::prune_older_than(db_pool, cutoff).await {
        tracing::error!(target: "slasha::metrics", error = ?err, "failed to prune old metrics");
    }

    Ok(())
}

fn compute_cpu_percent(cpu: Option<&ContainerCpuStats>, precpu: Option<&ContainerCpuStats>) -> f64 {
    let (cpu, precpu) = match (cpu, precpu) {
        (Some(c), Some(p)) => (c, p),
        _ => return 0.0,
    };

    let cpu_usage = match cpu.cpu_usage.as_ref() {
        Some(u) => u,
        None => return 0.0,
    };
    let precpu_usage = match precpu.cpu_usage.as_ref() {
        Some(u) => u,
        None => return 0.0,
    };

    let cpu_delta =
        cpu_usage.total_usage.unwrap_or(0) as f64 - precpu_usage.total_usage.unwrap_or(0) as f64;
    let system_delta =
        cpu.system_cpu_usage.unwrap_or(0) as f64 - precpu.system_cpu_usage.unwrap_or(0) as f64;

    if system_delta <= 0.0 || cpu_delta <= 0.0 {
        return 0.0;
    }

    let num_cpus = cpu.online_cpus.unwrap_or_else(|| {
        cpu_usage
            .percpu_usage
            .as_ref()
            .map(|v| v.len() as u32)
            .unwrap_or(1)
    }) as f64;

    (cpu_delta / system_delta) * num_cpus * 100.0
}

fn sum_network_bytes(networks: Option<&HashMap<String, ContainerNetworkStats>>) -> (u64, u64) {
    let Some(networks) = networks else {
        return (0, 0);
    };
    networks.values().fold((0, 0), |(rx, tx), n| {
        (rx + n.rx_bytes.unwrap_or(0), tx + n.tx_bytes.unwrap_or(0))
    })
}

fn sum_blkio_bytes(blkio: Option<&ContainerBlkioStats>) -> (u64, u64) {
    let Some(entries) = blkio.and_then(|b| b.io_service_bytes_recursive.as_ref()) else {
        return (0, 0);
    };
    entries.iter().fold((0, 0), |(read, write), entry| {
        let val = entry.value.unwrap_or(0);
        match entry
            .op
            .as_deref()
            .unwrap_or("")
            .to_ascii_lowercase()
            .as_str()
        {
            "read" => (read + val, write),
            "write" => (read, write + val),
            _ => (read, write),
        }
    })
}

fn bytes_to_mib(bytes: u64) -> i32 {
    (bytes / (1024 * 1024)) as i32
}
