use std::{
    collections::HashMap,
    time::{Duration, Instant},
};

use bollard::{
    Docker,
    plugin::{
        ContainerCreateBody, HealthConfig, HealthStatusEnum, HostConfig, Mount, MountType,
        NetworkingConfig, RestartPolicy, RestartPolicyNameEnum,
    },
    query_parameters::CreateContainerOptions,
};
use slasha_db::{app::App, service::Service};

use crate::{
    docker::{
        DockerError, DockerResult, labels,
        log_driver::default_log_config,
        naming::{app_network_name, service_container_name, service_volume_name},
        service::ServiceKindDockerExt,
    },
    logs::LogHandle,
};

/// Creates a database service process container in Docker.
///
/// # Arguments
///
/// * `docker_client` - Docker API client ([`Docker`]).
/// * `service` - Target database service model ([`Service`]).
/// * `app` - Target application model ([`App`]).
/// * `resolved_env` - Map of resolved environment key-value pairs.
pub async fn create_service_container(
    docker_client: &Docker,
    service: &Service,
    app: &App,
    resolved_env: &HashMap<String, String>,
) -> DockerResult<()> {
    let image_name = service
        .image_digest
        .clone()
        .unwrap_or_else(|| service.kind.docker_image(&service.version));

    let container_name = service_container_name(&service.id);
    let network_name = app_network_name(&app.id);
    let volume_name = service_volume_name(&service.id);

    let mut endpoints_config = HashMap::new();
    endpoints_config.insert(
        network_name.clone(),
        bollard::models::EndpointSettings {
            network_id: Some(network_name),
            aliases: Some(vec![service.name.clone()]),
            ..Default::default()
        },
    );

    let labels = labels::service_container_labels(app, &service.id);

    let env: Vec<String> = resolved_env
        .iter()
        .map(|(k, v)| format!("{}={}", k, v))
        .collect();

    let overrides = service.resources.clone().unwrap_or_default();
    let memory = overrides.memory_bytes;
    let nano_cpus = overrides.nano_cpus;
    let pids_limit = overrides.pids_limit;
    let shm_size = overrides.shm_size;

    docker_client
        .create_container(
            Some(CreateContainerOptions {
                name: Some(container_name.clone()),
                ..Default::default()
            }),
            ContainerCreateBody {
                image: Some(image_name),
                hostname: Some(container_name.clone()),
                labels: Some(labels),
                env: Some(env),
                cmd: service.kind.container_command(),
                healthcheck: Some(HealthConfig {
                    test: Some(service.kind.health_test()),
                    interval: Some(Duration::from_secs(5).as_nanos() as i64),
                    timeout: Some(Duration::from_secs(5).as_nanos() as i64),
                    retries: Some(10),
                    start_period: Some(Duration::from_secs(60).as_nanos() as i64),
                    start_interval: Some(Duration::from_secs(2).as_nanos() as i64),
                }),
                networking_config: Some(NetworkingConfig {
                    endpoints_config: Some(endpoints_config),
                }),
                host_config: Some(HostConfig {
                    restart_policy: Some(RestartPolicy {
                        name: Some(RestartPolicyNameEnum::UNLESS_STOPPED),
                        maximum_retry_count: None,
                    }),
                    mounts: Some(vec![Mount {
                        typ: Some(MountType::VOLUME),
                        source: Some(volume_name),
                        target: Some(service.kind.volume_mount_path().to_string()),
                        ..Default::default()
                    }]),
                    log_config: Some(default_log_config()),
                    memory,
                    nano_cpus,
                    pids_limit,
                    shm_size,
                    ..Default::default()
                }),
                ..Default::default()
            },
        )
        .await?;

    tracing::info!(
        container = %container_name,
        app_id = %app.id,
        service_id = %service.id,
        "service container created"
    );

    Ok(())
}

/// Polls a service container's health status via Docker inspection until it is reported healthy or fails.
///
/// # Arguments
///
/// * `docker_client` - Docker API client ([`Docker`]).
/// * `container_name` - Target container name.
/// * `service_name` - Service display name.
/// * `timeout_secs` - Timeout duration limit in seconds.
/// * `log` - Optional log handle reference ([`LogHandle`]).
pub async fn wait_for_service_health(
    docker_client: &Docker,
    container_name: &str,
    service_name: &str,
    timeout_secs: u64,
    log: Option<&LogHandle>,
) -> DockerResult<()> {
    if let Some(log) = log {
        let _ = log
            .send(format!("Waiting for {} to become healthy...", service_name))
            .await;
    }

    let deadline = Instant::now() + Duration::from_secs(timeout_secs);
    let mut last_status: Option<HealthStatusEnum> = None;

    loop {
        let inspect = docker_client
            .inspect_container(container_name, None)
            .await?;

        let status = inspect
            .state
            .as_ref()
            .and_then(|s| s.health.as_ref())
            .and_then(|h| h.status);

        if status != last_status {
            if let Some(s) = status
                && let Some(log) = log
            {
                let _ = log.send(format!("Health: {}", s)).await;
            }

            last_status = status;
        }

        match status {
            Some(HealthStatusEnum::HEALTHY) => return Ok(()),
            Some(HealthStatusEnum::UNHEALTHY) => {
                return Err(DockerError::ServiceHealthcheckFailed(
                    service_name.to_string(),
                ));
            }
            _ => {}
        }

        if Instant::now() >= deadline {
            return Err(DockerError::ServiceHealthcheckTimeout(
                service_name.to_string(),
                timeout_secs,
            ));
        }

        tokio::time::sleep(Duration::from_secs(2)).await;
    }
}
