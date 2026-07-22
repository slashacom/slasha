use std::{collections::HashMap, time::Duration};

use bollard::{
    Docker,
    models::{
        ContainerCreateBody, ContainerInspectResponse, EndpointSettings, HostConfig, Mount,
        MountType, NetworkCreateRequest, NetworkingConfig, PortBinding, RestartPolicy,
        RestartPolicyNameEnum, VolumeCreateRequest,
    },
    query_parameters::{
        CreateContainerOptions, CreateImageOptions, RemoveContainerOptionsBuilder,
        StartContainerOptionsBuilder,
    },
};
use futures_util::StreamExt;
use tokio::{net::TcpStream, time::sleep, time::timeout};

use super::{ProxyError, ProxyResult};
use crate::docker::log_driver::default_log_config;

pub const PROXY_CONTAINER_NAME: &str = "slasha-proxy";
pub const PROXY_NETWORK_NAME: &str = "slasha-proxy";
const IMAGE: &str = "caddy:latest";
const ADMIN_URL: &str = "http://127.0.0.1:2019/config/";
const HOST_PORTS: [u16; 3] = [80, 443, 2019];

pub async fn ensure_caddy_ready(docker: &Docker) -> ProxyResult<()> {
    if let Ok(existing) = docker.inspect_container(PROXY_CONTAINER_NAME, None).await {
        let running = existing.state.as_ref().and_then(|s| s.running) == Some(true);

        if running && ports_published(&existing) {
            return wait_for_admin_api().await;
        }

        if !running {
            docker
                .start_container(
                    PROXY_CONTAINER_NAME,
                    Some(StartContainerOptionsBuilder::new().build()),
                )
                .await
                .map_err(map_port_bind_error)?;

            let started = docker.inspect_container(PROXY_CONTAINER_NAME, None).await?;
            if ports_published(&started) {
                return wait_for_admin_api().await;
            }
        }

        tracing::warn!(
            container = %PROXY_CONTAINER_NAME,
            "proxy container is up but its host ports are not published — recreating"
        );
        docker
            .remove_container(
                PROXY_CONTAINER_NAME,
                Some(RemoveContainerOptionsBuilder::new().force(true).build()),
            )
            .await?;
    }

    ensure_host_ports_free().await?;

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
                    Mount { typ: Some(MountType::VOLUME), source: Some("slasha-caddy-data".into()),   target: Some("/data".into()),   ..Default::default() },
                    Mount { typ: Some(MountType::VOLUME), source: Some("slasha-caddy-config".into()), target: Some("/config".into()), ..Default::default() },
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
        .await
        .map_err(map_port_bind_error)?;

    wait_for_admin_api().await
}

fn ports_published(container: &ContainerInspectResponse) -> bool {
    let Some(ports) = container
        .network_settings
        .as_ref()
        .and_then(|n| n.ports.as_ref())
    else {
        return false;
    };

    HOST_PORTS.iter().all(|port| {
        ports
            .get(&format!("{port}/tcp"))
            .and_then(|bindings| bindings.as_ref())
            .is_some_and(|bindings| !bindings.is_empty())
    })
}

async fn ensure_host_ports_free() -> ProxyResult<()> {
    for port in HOST_PORTS {
        let probe = timeout(
            Duration::from_millis(300),
            TcpStream::connect(("127.0.0.1", port)),
        )
        .await;

        if matches!(probe, Ok(Ok(_))) {
            return Err(ProxyError::PortConflict(port));
        }
    }

    Ok(())
}

fn map_port_bind_error(e: bollard::errors::Error) -> ProxyError {
    let bollard::errors::Error::DockerResponseServerError { message, .. } = &e else {
        return ProxyError::DockerApi(e);
    };

    if message.contains("address already in use") || message.contains("port is already allocated")
    {
        return ProxyError::Caddy(format!(
            "cannot start the proxy container: {message} — another process on the host is using a port Caddy needs (80, 443 or 2019); find it with `sudo ss -ltnp | grep -E ':80|:443|:2019'`"
        ));
    }

    ProxyError::DockerApi(e)
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

    Err(ProxyError::Timeout(format!(
        "Caddy admin API did not become ready within 10s — check `docker logs {PROXY_CONTAINER_NAME}`"
    )))
}
