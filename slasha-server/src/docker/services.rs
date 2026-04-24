use bollard::Docker;
use bollard::models::{
    ContainerCreateBody, EndpointSettings, HostConfig, Mount, MountTypeEnum, NetworkingConfig,
    RestartPolicy, RestartPolicyNameEnum, VolumeCreateRequest,
};
use bollard::query_parameters::CreateImageOptions;
use bollard::query_parameters::{
    CreateContainerOptions, RemoveContainerOptionsBuilder, StartContainerOptionsBuilder,
    StopContainerOptionsBuilder,
};
use chrono::Utc;
use diesel::prelude::*;
use diesel::r2d2::{ConnectionManager, Pool};
use diesel::sqlite::SqliteConnection;
use futures_util::StreamExt;
use std::collections::HashMap;

use models::app::App;
use models::schema::{service_env_vars, services};
use models::service::{Service, ServiceEnvVar, ServiceStatus};

use crate::docker::env::{EnvRef, RefSource, parse_env_ref};
use crate::docker::network::app_network_name;
use crate::error::{DeploymentError, Result};

pub fn service_container_name(service_id: &str) -> String {
    format!("slasha-svc-{}", service_id)
}

fn service_volume_name(service_id: &str) -> String {
    format!("slasha-vol-{}", service_id)
}

pub fn update_service_status(
    conn: &mut SqliteConnection,
    service_id: &str,
    status: ServiceStatus,
) -> Result<()> {
    diesel::update(services::table.filter(services::id.eq(service_id)))
        .set((
            services::status.eq(status.to_string()),
            services::updated_at.eq(Utc::now().naive_utc()),
        ))
        .execute(conn)
        .map_err(DeploymentError::DatabaseError)?;

    Ok(())
}

pub fn resolve_service_env(
    db_pool: &Pool<ConnectionManager<SqliteConnection>>,
    service: &Service,
) -> Result<HashMap<String, String>> {
    let mut conn = db_pool.get().map_err(DeploymentError::PoolError)?;

    let vars: Vec<ServiceEnvVar> = service_env_vars::table
        .filter(service_env_vars::service_id.eq(&service.id))
        .order(service_env_vars::key.asc())
        .load(&mut conn)?;

    let raw_env: HashMap<String, String> = vars
        .iter()
        .map(|v| (v.key.clone(), v.value.clone()))
        .collect();

    let mut resolved = HashMap::with_capacity(vars.len());

    for var in &vars {
        let value = match parse_env_ref(&var.value) {
            EnvRef::Literal => var.value.clone(),
            EnvRef::Ref(RefSource::Own, key) => raw_env
                .get(&key)
                .cloned()
                .ok_or_else(|| DeploymentError::EnvResolveFailed(key))?,
            EnvRef::Ref(RefSource::System, key) => match key.as_str() {
                "service_container_name" => service_container_name(&service.id),
                "service_id" => service.id.clone(),
                "service_name" => service.name.clone(),
                "app_id" => service.app_id.clone(),
                "network_name" => app_network_name(&service.app_id),
                _ => {
                    return Err(DeploymentError::EnvResolveFailed(format!(
                        "Unknown system key: {}",
                        key
                    ))
                    .into());
                }
            },
            _ => var.value.clone(),
        };
        resolved.insert(var.key.clone(), value);
    }

    Ok(resolved)
}

pub async fn provision_service(
    docker: &Docker,
    db_pool: &Pool<ConnectionManager<SqliteConnection>>,
    app: &App,
    service: &Service,
    env_vars: HashMap<String, String>,
) -> Result<()> {
    let image_name = service.kind.docker_image(&service.version);

    let mut image_stream = docker.create_image(
        Some(CreateImageOptions {
            from_image: Some(image_name.clone()),
            ..Default::default()
        }),
        None,
        None,
    );

    while let Some(msg) = image_stream.next().await {
        let _ = msg.map_err(DeploymentError::DockerApi)?;
    }

    let volume_name = service_volume_name(&service.id);
    let vol_config = VolumeCreateRequest {
        name: Some(volume_name.clone()),
        ..Default::default()
    };
    docker
        .create_volume(vol_config)
        .await
        .map_err(DeploymentError::DockerApi)?;

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

    let mut conn = db_pool.get().map_err(DeploymentError::PoolError)?;

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

    diesel::insert_into(service_env_vars::table)
        .values(&new_vars)
        .execute(&mut conn)
        .map_err(DeploymentError::DatabaseError)?;

    let env_vars = resolve_service_env(db_pool, service)?;

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

    docker
        .create_container(Some(create_opts), config)
        .await
        .map_err(DeploymentError::DockerApi)?;

    docker
        .start_container(
            &container_name,
            Some(StartContainerOptionsBuilder::new().build()),
        )
        .await
        .map_err(DeploymentError::DockerApi)?;

    update_service_status(&mut conn, &service.id, ServiceStatus::Running)?;

    Ok(())
}

pub async fn stop_service(
    docker: &Docker,
    db_pool: &Pool<ConnectionManager<SqliteConnection>>,
    service: &Service,
) -> Result<()> {
    let container_name = service_container_name(&service.id);

    docker
        .stop_container(
            &container_name,
            Some(StopContainerOptionsBuilder::new().t(10).build()),
        )
        .await
        .map_err(DeploymentError::DockerApi)?;

    let mut conn = db_pool.get().map_err(DeploymentError::PoolError)?;
    update_service_status(&mut conn, &service.id, ServiceStatus::Stopped)?;

    Ok(())
}

pub async fn delete_service(
    docker: &Docker,
    db_pool: &Pool<ConnectionManager<SqliteConnection>>,
    service: &Service,
) -> Result<()> {
    let container_name = service_container_name(&service.id);
    let volume_name = service_volume_name(&service.id);

    if let Err(e) = docker
        .remove_container(
            &container_name,
            Some(RemoveContainerOptionsBuilder::new().force(true).build()),
        )
        .await
    {
        return Err(DeploymentError::DockerApi(e).into());
    }

    if let Err(e) = docker
        .remove_volume(
            &volume_name,
            None::<bollard::query_parameters::RemoveVolumeOptions>,
        )
        .await
    {
        return Err(DeploymentError::DockerApi(e).into());
    }

    let mut conn = db_pool.get().map_err(DeploymentError::PoolError)?;
    diesel::delete(services::table.filter(services::id.eq(&service.id)))
        .execute(&mut conn)
        .map_err(DeploymentError::DatabaseError)?;

    Ok(())
}
