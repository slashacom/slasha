use std::{
    collections::HashMap,
    sync::Arc,
    time::{Duration, Instant},
};

use bollard::{
    Docker,
    models::{
        EndpointSettings, HealthConfig, HealthStatusEnum, HostConfig, Mount, MountType,
        NetworkingConfig, ProgressDetail, RestartPolicy, RestartPolicyNameEnum,
        VolumeCreateRequest,
    },
    query_parameters::{
        CreateContainerOptions, CreateImageOptions, RemoveContainerOptionsBuilder,
        StartContainerOptionsBuilder,
    },
};
use futures_util::StreamExt;
use slasha_db::{
    DbPool,
    app::App,
    repos::service::ServiceRepo,
    service::{NewServiceEnvVar, Service, ServiceEnvVar, ServiceStatus},
};
use tokio::time::sleep;

use crate::{
    docker::{
        DeploymentError, DeploymentResult,
        env::{RefSource, resolve_env_value, topo_sort_vars},
        log_driver::default_log_config,
        naming::{app_network_name, service_container_name, service_volume_name},
        rollback::Rollback,
    },
    logs::{LogHandle, LogKey, LogManager, stream_container_logs},
};

pub fn resolve_env_vars(
    service_vars: Vec<ServiceEnvVar>,
    service: &Service,
) -> DeploymentResult<HashMap<String, String>> {
    let sorted = topo_sort_vars(service_vars, |v| &v.key, |v| &v.value)?;
    let mut resolved: HashMap<String, String> = HashMap::with_capacity(sorted.len());

    for var in sorted {
        let value = resolve_env_value(&var.value, |source, key| match source {
            RefSource::Own => Ok(resolved.get(key).unwrap().clone()),
            RefSource::System => match key {
                "service_container_name" => Ok(service_container_name(&service.id)),
                "service_id" => Ok(service.id.clone()),
                "service_name" => Ok(service.name.clone()),
                "app_id" => Ok(service.app_id.clone()),
                "network_name" => Ok(app_network_name(&service.app_id)),
                _ => Err(DeploymentError::EnvResolveFailed(format!(
                    "Unknown system key: {}",
                    key
                ))),
            },
            RefSource::Service(_) => Err(DeploymentError::EnvResolveFailed(
                "Service references not supported in this context".to_string(),
            )),
        })?;
        resolved.insert(var.key.clone(), value);
    }

    Ok(resolved)
}

pub async fn provision_service(
    docker: Docker,
    db_pool: DbPool,
    log_manager: Arc<LogManager>,
    app: App,
    service: Service,
    initial_env: Option<HashMap<String, String>>,
) -> DeploymentResult<()> {
    let log_key = LogKey::Service {
        app_slug: app.slug.clone(),
        service_name: service.name.clone(),
    };
    let log = log_manager.get_logger(&log_key).await?;
    let mut rollback = Rollback::new();

    if let Err(e) = provision_inner(
        &docker,
        &db_pool,
        &app,
        &service,
        initial_env,
        &log,
        &mut rollback,
    )
    .await
    {
        tracing::error!(
            app_id = %service.app_id,
            service_id = %service.id,
            service_name = %service.name,
            error = ?e,
            "Service provision failed"
        );
        let _ = log.send(format!("Service provision failed: {}", e)).await;
        rollback.execute().await;
        let _ = ServiceRepo::update_status(&db_pool, &service.id, ServiceStatus::Failed).await;
        log_manager.remove(&log_key);
        return Err(e);
    }

    rollback.disarm();
    Ok(())
}

async fn provision_inner(
    docker: &Docker,
    db_pool: &DbPool,
    app: &App,
    service: &Service,
    initial_env: Option<HashMap<String, String>>,
    log: &LogHandle,
    rollback: &mut Rollback,
) -> DeploymentResult<()> {
    log.send(format!(
        "Provisioning service {} ({})",
        service.name, service.kind
    ))
    .await?;

    ServiceRepo::update_status(db_pool, &service.id, ServiceStatus::Provisioning).await?;

    let mut stream = docker.create_image(
        Some(CreateImageOptions {
            from_image: Some(service.kind.docker_image(&service.version)),
            ..Default::default()
        }),
        None,
        None,
    );

    while let Some(result) = stream.next().await {
        let info = result?;
        if let Some(status) = info.status {
            let msg = match info.progress_detail {
                Some(ProgressDetail {
                    current: Some(current),
                    total: Some(total),
                }) => {
                    format!("{}: {}/{}", status, current, total)
                }
                _ => status,
            };
            log.send(msg).await?;
        }
    }

    let volume_name = service_volume_name(&service.id);
    docker
        .create_volume(VolumeCreateRequest {
            name: Some(volume_name.clone()),
            ..Default::default()
        })
        .await?;

    rollback.register({
        let docker = docker.clone();
        let volume_name = volume_name.clone();
        move || {
            Box::pin(async move {
                if let Err(e) = docker
                    .remove_volume(
                        &volume_name,
                        None::<bollard::query_parameters::RemoveVolumeOptions>,
                    )
                    .await
                {
                    tracing::warn!(volume = %volume_name, error = ?e, "Failed to remove volume");
                }
            })
        }
    });

    let env_vars = if let Some(env) = initial_env {
        let vars: Vec<NewServiceEnvVar> = env
            .into_iter()
            .map(|(key, value)| NewServiceEnvVar {
                service_id: service.id.clone(),
                key,
                value,
            })
            .collect();

        ServiceRepo::set_env_vars(db_pool, &service.id, vars).await?
    } else {
        ServiceRepo::get_env_vars(db_pool, &service.id).await?
    };

    let resolved_vars = resolve_env_vars(env_vars, service)?;

    create_service_container(docker, service, app, &resolved_vars, rollback).await?;
    start_and_wait_healthy(docker, service, log).await?;
    ServiceRepo::update_status(db_pool, &service.id, ServiceStatus::Running).await?;
    Ok(())
}

async fn create_service_container(
    docker_client: &Docker,
    service: &Service,
    app: &App,
    resolved_env: &HashMap<String, String>,
    rollback: &mut Rollback,
) -> DeploymentResult<()> {
    let image_name = service.kind.docker_image(&service.version);
    let container_name = service_container_name(&service.id);
    let network_name = app_network_name(&app.id);
    let volume_name = service_volume_name(&service.id);

    let mut endpoints_config = HashMap::new();
    endpoints_config.insert(
        network_name.clone(),
        EndpointSettings {
            network_id: Some(network_name),
            ..Default::default()
        },
    );

    let mut labels = HashMap::new();
    labels.insert("slasha.managed".to_string(), "true".to_string());
    labels.insert("slasha.app_id".to_string(), app.id.clone());
    labels.insert("slasha.service_id".to_string(), service.id.clone());

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
            bollard::models::ContainerCreateBody {
                image: Some(image_name),
                hostname: Some(container_name.clone()),
                labels: Some(labels),
                env: Some(env),
                cmd: service.kind.command(),
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
        "container created"
    );

    rollback.register({
        let container_name = container_name.to_string();
        let docker_client = docker_client.clone();

        move || {
            Box::pin(async move {
                if let Err(e) = docker_client
                    .remove_container(
                        &container_name,
                        Some(RemoveContainerOptionsBuilder::new().force(true).build()),
                    )
                    .await
                {
                    tracing::warn!(container = %container_name, error = ?e, "Failed to remove container");
                }
            })
        }
    });

    Ok(())
}

async fn start_and_wait_healthy(
    docker_client: &Docker,
    service: &Service,
    log: &LogHandle,
) -> DeploymentResult<()> {
    let container_name = service_container_name(&service.id);

    docker_client
        .start_container(
            &container_name,
            Some(StartContainerOptionsBuilder::new().build()),
        )
        .await?;

    stream_container_logs(
        docker_client.clone(),
        log.clone(),
        container_name.clone(),
        None,
    );

    wait_until_healthy(docker_client, &container_name, &service.name, log).await
}

const HEALTHCHECK_TIMEOUT_SECS: u64 = 180;

async fn wait_until_healthy(
    docker: &Docker,
    container_name: &str,
    service_name: &str,
    log: &LogHandle,
) -> DeploymentResult<()> {
    let _ = log
        .send(format!("Waiting for {} to become healthy...", service_name))
        .await;

    let deadline = Instant::now() + Duration::from_secs(HEALTHCHECK_TIMEOUT_SECS);
    let mut last_status: Option<HealthStatusEnum> = None;

    loop {
        let inspect = docker.inspect_container(container_name, None).await?;
        let status = inspect
            .state
            .as_ref()
            .and_then(|s| s.health.as_ref())
            .and_then(|h| h.status);

        if status != last_status {
            if let Some(s) = status {
                let _ = log.send(format!("Health: {}", s)).await;
            }
            last_status = status;
        }

        match status {
            Some(HealthStatusEnum::HEALTHY) => return Ok(()),
            Some(HealthStatusEnum::UNHEALTHY) => {
                return Err(DeploymentError::HealthcheckFailed(service_name.to_string()));
            }
            _ => {}
        }

        if Instant::now() >= deadline {
            return Err(DeploymentError::HealthcheckTimeout(
                service_name.to_string(),
                HEALTHCHECK_TIMEOUT_SECS,
            ));
        }

        sleep(Duration::from_secs(2)).await;
    }
}
