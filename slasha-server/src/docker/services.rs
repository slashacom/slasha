use std::collections::HashMap;

use bollard::{
    Docker,
    models::{
        ContainerCreateBody, EndpointSettings, HostConfig, Mount, MountTypeEnum, NetworkingConfig,
        RestartPolicy, RestartPolicyNameEnum, VolumeCreateRequest,
    },
    query_parameters::{
        CreateContainerOptions, CreateImageOptions, RemoveContainerOptionsBuilder,
        StartContainerOptionsBuilder, StopContainerOptionsBuilder,
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

use super::{
    DeploymentError, DeploymentResult,
    env::RefSource,
    logs::{LogKey, LogManager, stream_container_logs},
    network::app_network_name,
};
use crate::docker::env::{resolve_env_value, topo_sort_vars};

pub fn service_container_name(service_id: &str) -> String {
    format!("slasha-svc-{}", service_id)
}

fn service_volume_name(service_id: &str) -> String {
    format!("slasha-vol-{}", service_id)
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
    docker_client: &Docker,
    db_pool: &DbPool,
    log_manager: &LogManager,
    app: &App,
    service: &Service,
    env_vars: HashMap<String, String>,
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
    docker_client
        .create_volume(vol_config)
        .await?;

    let container_name = service_container_name(&service.id);
    let network_name = app_network_name(&app.id);

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

    let now = Utc::now().naive_utc();
    let new_vars: Vec<ServiceEnvVar> = env_vars
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

    ServiceRepo::insert_env_vars(db_pool, new_vars).await?;

    let service_vars = ServiceRepo::get_env_vars(db_pool, &service.id).await?;
    let env_vars = resolve_service_env(service_vars, service)?;

    let internal_env: Vec<String> = env_vars
        .into_iter()
        .map(|(k, v)| format!("{}={}", k, v))
        .collect();

    let mount_target = service.kind.volume_mount_path();

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
        ..Default::default()
    };

    let config = ContainerCreateBody {
        image: Some(image_name),
        hostname: Some(container_name.clone()),
        labels: Some(labels),
        host_config: Some(host_config),
        networking_config: Some(NetworkingConfig {
            endpoints_config: Some(endpoints_config),
        }),
        env: Some(internal_env),
        ..Default::default()
    };

    let create_opts = CreateContainerOptions {
        name: Some(container_name.clone()),
        ..Default::default()
    };

    docker_client
        .create_container(Some(create_opts), config)
        .await?;

    docker_client
        .start_container(
            &container_name,
            Some(StartContainerOptionsBuilder::new().build()),
        )
        .await?;

    ServiceRepo::update_status(db_pool, &service.id, ServiceStatus::Running).await?;

    let log_key = LogKey::Service {
        app_slug: app.slug.clone(),
        service_name: service.name.clone(),
    };
    let log = log_manager.get_logger(&log_key).await?;
    let docker_clone = docker_client.clone();
    let container_name_clone = container_name.clone();

    tokio::spawn(async move {
        if let Err(e) = stream_container_logs(docker_clone, log, container_name_clone).await {
            tracing::warn!("service log stream ended with error: {:?}", e);
        }
    });

    Ok(())
}

pub async fn stop_service(
    docker_client: &Docker,
    db_pool: &DbPool,
    log_manager: &LogManager,
    app: &App,
    service: &Service,
) -> DeploymentResult<()> {
    let container_name = service_container_name(&service.id);

    docker_client
        .stop_container(
            &container_name,
            Some(StopContainerOptionsBuilder::new().t(10).build()),
        )
        .await?;

    ServiceRepo::update_status(db_pool, &service.id, ServiceStatus::Stopped).await?;

    log_manager.remove(&LogKey::Service {
        app_slug: app.slug.clone(),
        service_name: service.name.clone(),
    });

    Ok(())
}

pub async fn delete_service(
    docker_client: &Docker,
    db_pool: &DbPool,
    log_manager: &LogManager,
    app: &App,
    service: &Service,
) -> DeploymentResult<()> {
    let container_name = service_container_name(&service.id);
    let volume_name = service_volume_name(&service.id);

    docker_client
        .remove_container(
            &container_name,
            Some(RemoveContainerOptionsBuilder::new().force(true).build()),
        )
        .await?;

    docker_client
        .remove_volume(
            &volume_name,
            None::<bollard::query_parameters::RemoveVolumeOptions>,
        )
        .await?;

    ServiceRepo::delete(db_pool, &service.id).await?;

    log_manager.remove(&LogKey::Service {
        app_slug: app.slug.clone(),
        service_name: service.name.clone(),
    });

    Ok(())
}
