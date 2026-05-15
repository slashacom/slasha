use std::{
    collections::HashMap,
    sync::Arc,
    time::{Duration, Instant},
};

use bollard::{
    Docker,
    models::{
        ContainerCreateBody, EndpointSettings, HealthConfig, HealthStatusEnum, HostConfig, Mount,
        MountTypeEnum, NetworkingConfig, PortBinding, RestartPolicy, RestartPolicyNameEnum,
        VolumeCreateRequest,
    },
    query_parameters::{
        CreateContainerOptions, CreateImageOptions, RemoveContainerOptionsBuilder,
        StartContainerOptionsBuilder,
    },
};
use chrono::Utc;
use futures_util::StreamExt;
use slasha_db::{
    DbPool,
    app::App,
    repos::service::ServiceRepo,
    service::{Service, ServiceEnvVar, ServiceStatus},
};
use tokio::time::sleep;

use crate::docker::{
    DeploymentError, DeploymentResult,
    env::{RefSource, resolve_env_value, topo_sort_vars},
    log_driver::default_log_config,
    logs::{Log, LogKey, LogManager, stream_container_logs},
    naming::{app_network_name, service_container_name, service_volume_name},
    rollback::Rollback,
};

#[derive(Debug, Clone)]
pub struct ExposureSpec {
    pub host_port: u16,
    pub bind_addr: String,
}

pub fn resolve_service_env(
    service_vars: Vec<ServiceEnvVar>,
    service: &Service,
) -> DeploymentResult<HashMap<String, String>> {
    let sorted_vars: Vec<ServiceEnvVar> = topo_sort_vars(service_vars, |v| &v.key, |v| &v.value)?;

    let mut resolved: HashMap<String, String> = HashMap::with_capacity(sorted_vars.len());

    for var in sorted_vars {
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
    docker_client: Docker,
    db_pool: DbPool,
    log_manager: Arc<LogManager>,
    app: App,
    service: Service,
    env_vars: HashMap<String, String>,
) -> DeploymentResult<()> {
    let log_key = LogKey::Service {
        app_slug: app.slug.clone(),
        service_name: service.name.clone(),
    };
    let log = log_manager.get_logger(&log_key).await?;
    let mut rollback = Rollback::new();

    if let Err(e) = provision_service_inner(
        &docker_client,
        &db_pool,
        &app,
        &service,
        env_vars,
        &log,
        &mut rollback,
    )
    .await
    {
        tracing::error!("Service provision failed: {:?}", e);
        let _ = log.send(format!("Service provision failed: {}", e)).await;

        rollback.execute().await;

        let _ = ServiceRepo::update_status(&db_pool, &service.id, ServiceStatus::Failed).await;
        log_manager.remove(&log_key);

        return Err(e);
    }

    rollback.disarm();

    Ok(())
}

async fn provision_service_inner(
    docker_client: &Docker,
    db_pool: &DbPool,
    app: &App,
    service: &Service,
    env_vars: HashMap<String, String>,
    log: &Log,
    rollback: &mut Rollback,
) -> DeploymentResult<()> {
    let image_name = service.kind.docker_image(&service.version);

    let mut image_stream = docker_client.create_image(
        Some(CreateImageOptions {
            from_image: Some(image_name.clone()),
            ..Default::default()
        }),
        None,
        None,
    );

    while let Some(result) = image_stream.next().await {
        result?;
    }

    let volume_name = service_volume_name(&service.id);
    let vol_config = VolumeCreateRequest {
        name: Some(volume_name.clone()),
        ..Default::default()
    };
    docker_client.create_volume(vol_config).await?;

    rollback.register({
        let docker_client = docker_client.clone();
        let volume_name = volume_name.clone();

        move || {
            Box::pin(async move {
                let _ = docker_client
                    .remove_volume(
                        &volume_name,
                        None::<bollard::query_parameters::RemoveVolumeOptions>,
                    )
                    .await;
            })
        }
    });

    let now = Utc::now().naive_utc();
    let initial_vars: Vec<ServiceEnvVar> = env_vars
        .into_iter()
        .map(|(key, value)| ServiceEnvVar {
            id: uuid::Uuid::new_v4().to_string(),
            service_id: service.id.clone(),
            key,
            value,
            created_at: now,
            updated_at: now,
        })
        .collect();

    let resolved_map = resolve_service_env(initial_vars.clone(), service)?;

    let new_vars: Vec<ServiceEnvVar> = resolved_map
        .clone()
        .into_iter()
        .map(|(key, value)| {
            let id = initial_vars
                .iter()
                .find(|v| v.key == key)
                .unwrap()
                .id
                .clone();
            ServiceEnvVar {
                id,
                service_id: service.id.clone(),
                key,
                value,
                created_at: now,
                updated_at: now,
            }
        })
        .collect();

    ServiceRepo::insert_env_vars(db_pool, new_vars).await?;

    let body = build_service_container_body(service, app, &resolved_map, None);

    create_start_and_wait_healthy(
        docker_client,
        db_pool,
        service,
        body,
        log,
        Some(rollback),
    )
    .await
}

pub(crate) fn build_service_container_body(
    service: &Service,
    app: &App,
    resolved_env: &HashMap<String, String>,
    exposure: Option<&ExposureSpec>,
) -> ContainerCreateBody {
    let image_name = service.kind.docker_image(&service.version);
    let container_name = service_container_name(&service.id);
    let network_name = app_network_name(&app.id);
    let volume_name = service_volume_name(&service.id);
    let mount_target = service.kind.volume_mount_path();

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

    let port_bindings = exposure.map(|spec| {
        let mut bindings = HashMap::new();
        bindings.insert(
            format!("{}/tcp", service.kind.container_port()),
            Some(vec![PortBinding {
                host_ip: Some(spec.bind_addr.clone()),
                host_port: Some(spec.host_port.to_string()),
            }]),
        );
        bindings
    });

    let host_config = HostConfig {
        restart_policy: Some(RestartPolicy {
            name: Some(RestartPolicyNameEnum::UNLESS_STOPPED),
            maximum_retry_count: None,
        }),
        mounts: Some(vec![Mount {
            typ: Some(MountTypeEnum::VOLUME),
            source: Some(volume_name),
            target: Some(mount_target.to_string()),
            ..Default::default()
        }]),
        log_config: Some(default_log_config()),
        memory: Some(
            overrides
                .memory_bytes
                .unwrap_or_else(|| service.kind.default_memory_bytes()),
        ),
        nano_cpus: Some(
            overrides
                .nano_cpus
                .unwrap_or_else(|| service.kind.default_nano_cpus()),
        ),
        pids_limit: Some(
            overrides
                .pids_limit
                .unwrap_or_else(|| service.kind.default_pids_limit()),
        ),
        shm_size: Some(
            overrides
                .shm_size
                .unwrap_or_else(|| service.kind.default_shm_size()),
        ),
        port_bindings,
        ..Default::default()
    };

    let healthcheck = HealthConfig {
        test: Some(service.kind.health_test()),
        interval: Some(Duration::from_secs(5).as_nanos() as i64),
        timeout: Some(Duration::from_secs(5).as_nanos() as i64),
        retries: Some(10),
        start_period: Some(Duration::from_secs(60).as_nanos() as i64),
        start_interval: Some(Duration::from_secs(2).as_nanos() as i64),
    };

    ContainerCreateBody {
        image: Some(image_name),
        hostname: Some(container_name),
        labels: Some(labels),
        host_config: Some(host_config),
        networking_config: Some(NetworkingConfig {
            endpoints_config: Some(endpoints_config),
        }),
        env: Some(env),
        healthcheck: Some(healthcheck),
        cmd: service.kind.command(),
        ..Default::default()
    }
}

pub(crate) async fn create_start_and_wait_healthy(
    docker_client: &Docker,
    db_pool: &DbPool,
    service: &Service,
    body: ContainerCreateBody,
    log: &Log,
    rollback: Option<&mut Rollback>,
) -> DeploymentResult<()> {
    let container_name = service_container_name(&service.id);

    let create_opts = CreateContainerOptions {
        name: Some(container_name.clone()),
        ..Default::default()
    };

    docker_client
        .create_container(Some(create_opts), body)
        .await?;

    if let Some(rb) = rollback {
        rb.register({
            let docker_client = docker_client.clone();
            let container_name = container_name.clone();
            move || {
                Box::pin(async move {
                    let _ = docker_client
                        .remove_container(
                            &container_name,
                            Some(RemoveContainerOptionsBuilder::new().force(true).build()),
                        )
                        .await;
                })
            }
        });
    }

    docker_client
        .start_container(
            &container_name,
            Some(StartContainerOptionsBuilder::new().build()),
        )
        .await?;

    tokio::spawn({
        let docker_client = docker_client.clone();
        let container_name = container_name.clone();
        let log = log.clone();

        async move {
            if let Err(e) = stream_container_logs(docker_client, log, container_name, None).await {
                tracing::warn!("service log stream ended with error: {:?}", e);
            }
        }
    });

    wait_until_healthy(docker_client, &container_name, &service.name, log).await?;

    ServiceRepo::update_status(db_pool, &service.id, ServiceStatus::Running).await?;

    Ok(())
}

const HEALTHCHECK_TIMEOUT_SECS: u64 = 180;

async fn wait_until_healthy(
    docker_client: &Docker,
    container_name: &str,
    service_name: &str,
    log: &Log,
) -> DeploymentResult<()> {
    let _ = log
        .send(format!("Waiting for {} to report healthy...", service_name))
        .await;

    let deadline = Instant::now() + Duration::from_secs(HEALTHCHECK_TIMEOUT_SECS);
    let mut last_status: Option<HealthStatusEnum> = None;

    loop {
        let inspect = docker_client.inspect_container(container_name, None).await?;

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
