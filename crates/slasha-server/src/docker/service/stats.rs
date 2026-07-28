use bollard::{
    Docker,
    container::LogOutput,
    exec::{CreateExecOptions, StartExecOptions, StartExecResults},
    query_parameters::StatsOptionsBuilder,
};
use futures_util::StreamExt;
use serde::Serialize;
use slasha_db::service::Service;

use super::spec::ServiceKindDockerExt;
use crate::{docker::naming::service_container_name, metrics::app::compute_cpu_percent};

#[derive(Serialize)]
pub struct ServiceStats {
    pub running: bool,
    pub started_at: Option<String>,
    pub cpu_percent: Option<f64>,
    pub memory_used_bytes: Option<u64>,
    pub disk_bytes: Option<i64>,
}

/// Fetches resource usage statistics for a database service container.
///
/// # Arguments
///
/// * `docker_client` - Docker API client ([`Docker`]).
/// * `service` - Target database service model ([`Service`]).
///
/// # Returns
///
/// Option containing a [`ServiceStats`] struct.
pub async fn get_service_stats(docker_client: &Docker, service: &Service) -> Option<ServiceStats> {
    let container_name = service_container_name(&service.id);

    let state = docker_client
        .inspect_container(&container_name, None)
        .await
        .ok()
        .and_then(|info| info.state);

    let running = state.as_ref().and_then(|s| s.running).unwrap_or(false);
    let started_at = if running {
        state.as_ref().and_then(|s| s.started_at.clone())
    } else {
        None
    };

    let mut cpu_percent = None;
    let mut memory_used_bytes = None;

    if running {
        let opts = StatsOptionsBuilder::default()
            .stream(false)
            .one_shot(true)
            .build();

        if let Some(Ok(stats)) = docker_client
            .stats(&container_name, Some(opts))
            .next()
            .await
        {
            cpu_percent = Some(compute_cpu_percent(
                stats.cpu_stats.as_ref(),
                stats.precpu_stats.as_ref(),
            ));
            if let Some(mem) = stats.memory_stats.as_ref() {
                memory_used_bytes = mem.usage;
            }
        }
    }

    let disk_bytes = if running {
        service_disk_bytes(docker_client, service).await
    } else {
        None
    };

    Some(ServiceStats {
        running,
        started_at,
        cpu_percent,
        memory_used_bytes,
        disk_bytes,
    })
}

/// Measures disk usage of a database volume via `du -sk` execution in the container.
///
/// # Arguments
///
/// * `docker` - Docker API client ([`Docker`]).
/// * `service` - Target database service model ([`Service`]).
///
/// # Returns
///
/// Option containing disk usage in bytes (`i64`).
async fn service_disk_bytes(docker: &Docker, service: &Service) -> Option<i64> {
    let container_name = service_container_name(&service.id);
    let mount = service.kind.volume_mount_path();

    let exec = docker
        .create_exec(
            &container_name,
            CreateExecOptions {
                attach_stdout: Some(true),
                attach_stderr: Some(false),
                cmd: Some(vec!["du".to_string(), "-sk".to_string(), mount.to_string()]),
                ..Default::default()
            },
        )
        .await
        .ok()?;

    let mut output = match docker
        .start_exec(&exec.id, None::<StartExecOptions>)
        .await
        .ok()?
    {
        StartExecResults::Attached { output, .. } => output,
        StartExecResults::Detached => return None,
    };

    let mut buf = String::new();
    while let Some(Ok(chunk)) = output.next().await {
        if let LogOutput::StdOut { message } = chunk {
            buf.push_str(&String::from_utf8_lossy(&message));
        }
    }

    let kilobytes: i64 = buf.split_whitespace().next()?.parse().ok()?;
    Some(kilobytes * 1024)
}
