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

pub const PROXY_CONTAINER_NAME: &str = "slasha-proxy";
pub const PROXY_NETWORK_NAME: &str = "slasha-proxy";
const IMAGE: &str = "caddy:latest";
const ADMIN_URL: &str = "http://127.0.0.1:2019/config/";

pub async fn ensure_caddy_ready(docker_client: &Docker) -> ProxyResult<()> {
    ensure_proxy_network(docker_client).await?;
    ensure_caddy_volumes(docker_client).await?;
    pull_caddy_image(docker_client).await?;

    if !caddy_container_exists(docker_client).await {
        create_caddy_container(docker_client).await?;
    }

    start_caddy_container(docker_client).await;
    wait_for_caddy_ready().await?;

    Ok(())
}

pub async fn ensure_proxy_network(docker_client: &Docker) -> ProxyResult<()> {
    let config = NetworkCreateRequest {
        name: PROXY_NETWORK_NAME.to_string(),
        driver: Some("bridge".to_string()),
        ..Default::default()
    };

    match docker_client.create_network(config).await {
        Ok(_) => Ok(()),
        Err(bollard::errors::Error::DockerResponseServerError {
            status_code: 409, ..
        }) => Ok(()), // already exists
        Err(e) => Err(ProxyError::DockerApi(e)),
    }
}

async fn ensure_caddy_volumes(docker_client: &Docker) -> ProxyResult<()> {
    let volumes = ["slasha-caddy-data", "slasha-caddy-config"];

    for &name in &volumes {
        let req = VolumeCreateRequest {
            name: Some(name.to_string()),
            ..Default::default()
        };

        docker_client.create_volume(req).await?;
    }

    Ok(())
}

async fn pull_caddy_image(docker_client: &Docker) -> ProxyResult<()> {
    let mut stream = docker_client.create_image(
        Some(CreateImageOptions {
            from_image: Some(IMAGE.to_string()),
            ..Default::default()
        }),
        None,
        None,
    );

    while let Some(result) = stream.next().await {
        if let Err(e) = result {
            tracing::warn!("Failed to pull {}: {}", IMAGE, e);
        }
    }

    Ok(())
}

async fn caddy_container_exists(docker_client: &Docker) -> bool {
    docker_client
        .inspect_container(PROXY_CONTAINER_NAME, None)
        .await
        .is_ok()
}

async fn create_caddy_container(docker_client: &Docker) -> ProxyResult<()> {
    let mut labels = HashMap::new();
    labels.insert("slasha.managed".into(), "true".into());
    labels.insert("slasha.role".into(), "proxy".into());

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

    let host_config = HostConfig {
        port_bindings: Some(port_bindings),
        extra_hosts: Some(vec!["host.docker.internal:host-gateway".to_string()]),
        restart_policy: Some(RestartPolicy {
            name: Some(RestartPolicyNameEnum::UNLESS_STOPPED),
            ..Default::default()
        }),
        mounts: Some(vec![
            mount_volume("slasha-caddy-data", "/data"),
            mount_volume("slasha-caddy-config", "/config"),
        ]),
        ..Default::default()
    };

    let mut endpoints_config = HashMap::new();
    endpoints_config.insert(
        PROXY_NETWORK_NAME.to_string(),
        EndpointSettings {
            network_id: Some(PROXY_NETWORK_NAME.to_string()),
            ..Default::default()
        },
    );

    let config = ContainerCreateBody {
        image: Some(IMAGE.to_string()),
        labels: Some(labels),
        host_config: Some(host_config),
        networking_config: Some(NetworkingConfig {
            endpoints_config: Some(endpoints_config),
        }),
        cmd: Some(vec![
            "/bin/sh".to_string(),
            "-c".to_string(),
            "printf '{\n  admin 0.0.0.0:2019\n}\n' > /etc/caddy/Caddyfile && caddy run --config /etc/caddy/Caddyfile --adapter caddyfile".to_string(),
        ]),
        ..Default::default()
    };

    docker_client
        .create_container(
            Some(CreateContainerOptions {
                name: Some(PROXY_CONTAINER_NAME.to_string()),
                ..Default::default()
            }),
            config,
        )
        .await?;

    Ok(())
}

fn mount_volume(source: &str, target: &str) -> Mount {
    Mount {
        typ: Some(MountTypeEnum::VOLUME),
        source: Some(source.to_string()),
        target: Some(target.to_string()),
        ..Default::default()
    }
}

async fn start_caddy_container(docker_client: &Docker) {
    let opts = StartContainerOptionsBuilder::new().build();

    if let Err(e) = docker_client
        .start_container(PROXY_CONTAINER_NAME, Some(opts))
        .await
    {
        tracing::debug!("Container start result (may already be running): {:?}", e);
    }
}

async fn wait_for_caddy_ready() -> ProxyResult<()> {
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
        "Caddy admin API did not become ready within 20s".into(),
    ))
}
