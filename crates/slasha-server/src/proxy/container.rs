use std::{collections::HashMap, time::Duration};

use bollard::{
    Docker,
    models::{
        ContainerCreateBody, EndpointSettings, HostConfig, Mount, MountTypeEnum,
        NetworkCreateRequest, NetworkingConfig, PortBinding, RestartPolicy, RestartPolicyNameEnum,
        VolumeCreateRequest,
    },
    query_parameters::{CreateContainerOptions, CreateImageOptions, StartContainerOptionsBuilder},
};
use futures_util::StreamExt;
use tokio::time::sleep;

use super::{ProxyError, ProxyResult};
use crate::docker::log_driver::default_log_config;

pub const PROXY_CONTAINER_NAME: &str = "slasha-proxy";
pub const PROXY_NETWORK_NAME: &str = "slasha-proxy";
const IMAGE: &str = "caddy:latest";
const ADMIN_URL: &str = "http://127.0.0.1:2019/config/";

pub async fn ensure_caddy_ready(docker: &Docker) -> ProxyResult<()> {
    let state = docker
        .inspect_container(PROXY_CONTAINER_NAME, None)
        .await
        .ok()
        .and_then(|c| c.state);

    if let Some(s) = state {
        // container exists, just start it
        if s.running != Some(true) {
            docker
                .start_container(
                    PROXY_CONTAINER_NAME,
                    Some(StartContainerOptionsBuilder::new().build()),
                )
                .await?;
        }

        // container running already
        return wait_for_admin_api().await;
    }

    match docker
        .create_network(NetworkCreateRequest {
            name: PROXY_NETWORK_NAME.to_string(),
            driver: Some("bridge".to_string()),
            ..Default::default()
        })
        .await
    {
        // network already exists
        Ok(_)
        | Err(bollard::errors::Error::DockerResponseServerError {
            status_code: 409, ..
        }) => {}
        Err(e) => return Err(ProxyError::DockerApi(e)),
    }

    for name in ["slasha-caddy-data", "slasha-caddy-config"] {
        docker
            .create_volume(VolumeCreateRequest {
                name: Some(name.to_string()),
                ..Default::default()
            })
            .await?;
    }

    let mut stream = docker.create_image(
        Some(CreateImageOptions {
            from_image: Some(IMAGE.to_string()),
            ..Default::default()
        }),
        None,
        None,
    );

    while let Some(result) = stream.next().await {
        if let Err(e) = result {
            tracing::warn!(
                image = %IMAGE,
                error = ?e,
                "Failed to pull image"
            );
        }
    }

    let mut port_bindings: HashMap<String, Option<Vec<PortBinding>>> = HashMap::new();

    port_bindings.insert(
        "80/tcp".into(),
        Some(vec![PortBinding {
            host_ip: None,
            host_port: Some("80".into()),
        }]),
    );
    port_bindings.insert(
        "443/tcp".into(),
        Some(vec![PortBinding {
            host_ip: None,
            host_port: Some("443".into()),
        }]),
    );
    port_bindings.insert(
        "2019/tcp".into(),
        Some(vec![PortBinding {
            host_ip: Some("127.0.0.1".into()),
            host_port: Some("2019".into()),
        }]),
    );

    let mut labels = HashMap::new();
    labels.insert("slasha.managed".into(), "true".into());
    labels.insert("slasha.role".into(), "proxy".into());

    docker.create_container(
        Some(CreateContainerOptions { name: Some(PROXY_CONTAINER_NAME.to_string()), ..Default::default() }),
        ContainerCreateBody {
            image: Some(IMAGE.to_string()),
            labels: Some(labels),
            cmd: Some(vec![
                "/bin/sh".into(),
                "-c".into(),
                "printf '{\n  admin 0.0.0.0:2019\n}\n' > /etc/caddy/Caddyfile && caddy run --config /etc/caddy/Caddyfile --adapter caddyfile".into(),
            ]),
            host_config: Some(HostConfig {
                port_bindings: Some(port_bindings),
                extra_hosts: Some(vec!["host.docker.internal:host-gateway".into()]),
                restart_policy: Some(RestartPolicy {
                    name: Some(RestartPolicyNameEnum::UNLESS_STOPPED),
                    ..Default::default()
                }),
                mounts: Some(vec![
                    Mount { typ: Some(MountTypeEnum::VOLUME), source: Some("slasha-caddy-data".into()),   target: Some("/data".into()),   ..Default::default() },
                    Mount { typ: Some(MountTypeEnum::VOLUME), source: Some("slasha-caddy-config".into()), target: Some("/config".into()), ..Default::default() },
                ]),
                log_config: Some(default_log_config()),
                ..Default::default()
            }),
            networking_config: Some(NetworkingConfig {
                endpoints_config: Some(HashMap::from([(
                    PROXY_NETWORK_NAME.to_string(),
                    EndpointSettings { network_id: Some(PROXY_NETWORK_NAME.to_string()), ..Default::default() },
                )])),
            }),
            ..Default::default()
        },
    ).await?;

    tracing::info!(
        container = %PROXY_CONTAINER_NAME,
        "container created"
    );

    docker
        .start_container(
            PROXY_CONTAINER_NAME,
            Some(StartContainerOptionsBuilder::new().build()),
        )
        .await?;

    wait_for_admin_api().await
}

async fn wait_for_admin_api() -> ProxyResult<()> {
    let client = reqwest::Client::new();

    for _ in 0..20 {
        if let Ok(res) = client.get(ADMIN_URL).send().await
            && res.status().is_success()
        {
            return Ok(());
        }
        sleep(Duration::from_millis(500)).await;
    }

    Err(ProxyError::Timeout(
        "Caddy admin API did not become ready within 10s".into(),
    ))
}
