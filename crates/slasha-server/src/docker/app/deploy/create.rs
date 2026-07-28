use std::collections::HashMap;

use bollard::{
    Docker,
    models::{
        ContainerCreateBody, EndpointSettings, HostConfig, Mount, MountType, NetworkingConfig,
        RestartPolicy, RestartPolicyNameEnum,
    },
    query_parameters::CreateContainerOptions,
};
use slasha_db::{
    app::App, app_backup::AppBackup, deployment::Deployment, models::app_scale::ProcessType,
};

use crate::{
    docker::{
        DockerResult,
        app::{deploy::context::MANAGED_DATA_PATH, image::image_tag, litestream},
        labels::process_container_labels,
        log_driver::default_log_config,
        naming::{app_network_name, app_volume_name, process_container_name},
        utils,
    },
    proxy::container::PROXY_NETWORK_NAME,
};

pub struct CreateContainerContext<'a> {
    pub process_type: ProcessType,
    pub instance_index: u32,
    pub container_port: Option<u16>,
    pub cmd: Option<&'a str>,
    pub env_map: &'a HashMap<String, String>,
    pub volume_paths: &'a [String],
    pub backup: Option<&'a AppBackup>,
    pub litestream_volume: Option<&'a str>,
}

/// Creates a process container in Docker for a deployment.
///
/// # Arguments
///
/// * `docker_client` - Docker API client ([`Docker`]).
/// * `app` - Target application model ([`App`]).
/// * `deployment` - Target deployment model ([`Deployment`]).
/// * `context` - Container creation options ([`CreateContainerContext`]).
pub async fn create_process_container(
    docker_client: &Docker,
    app: &App,
    deployment: &Deployment,
    context: CreateContainerContext<'_>,
) -> DockerResult<()> {
    let container_name = process_container_name(
        &app.id,
        &deployment.id,
        &context.process_type,
        context.instance_index,
    );

    let mut mounts = build_mounts(docker_client, &app.id, context.volume_paths).await?;

    let mut cmd = context.cmd.map(|s| s.to_string());
    let mut env_map = context.env_map.clone();

    if let Some(backup) = &context.backup
        && backup.enabled
        && context.process_type == ProcessType::Web
        && context.instance_index == 0
    {
        match (&cmd, &context.litestream_volume) {
            (Some(original_cmd), Some(_volume)) => {
                let plan = litestream::plan(backup, original_cmd);
                cmd = Some(plan.command);
                env_map.extend(plan.env);
                mounts.push(litestream::binary_mount());
            }
            (None, _) => tracing::warn!(
                app_id = %app.id,
                "backups enabled but the web process has no start command; skipping replication"
            ),
            (_, None) => tracing::warn!(
                app_id = %app.id,
                "backups enabled but the litestream binary is unavailable; skipping replication"
            ),
        }
    }

    let labels = process_container_labels(
        app,
        deployment,
        &context.process_type,
        &context.instance_index,
        context.container_port,
    );

    let env: Option<Vec<String>> = if env_map.is_empty() {
        None
    } else {
        Some(
            env_map
                .into_iter()
                .map(|(k, v)| format!("{}={}", k, v))
                .collect(),
        )
    };

    let app_network = app_network_name(&app.id);
    let mut endpoints_config = HashMap::new();
    endpoints_config.insert(
        app_network.clone(),
        EndpointSettings {
            network_id: Some(app_network),
            ..Default::default()
        },
    );

    if context.process_type == ProcessType::Web {
        endpoints_config.insert(
            PROXY_NETWORK_NAME.to_string(),
            EndpointSettings {
                network_id: Some(PROXY_NETWORK_NAME.to_string()),
                ..Default::default()
            },
        );
    }

    let (entrypoint, container_cmd) = match cmd {
        Some(c) => (
            Some(vec!["sh".to_string(), "-c".to_string()]),
            Some(vec![c]),
        ),
        None => (None, None),
    };

    docker_client
        .create_container(
            Some(CreateContainerOptions {
                name: Some(container_name.clone()),
                ..Default::default()
            }),
            ContainerCreateBody {
                image: Some(image_tag(&app.slug, &deployment.id)),
                labels: Some(labels),
                env,
                entrypoint,
                cmd: container_cmd,
                host_config: Some(HostConfig {
                    restart_policy: Some(match context.process_type {
                        ProcessType::Release => RestartPolicy {
                            name: Some(RestartPolicyNameEnum::EMPTY),
                            maximum_retry_count: None,
                        },
                        _ => RestartPolicy {
                            name: Some(RestartPolicyNameEnum::UNLESS_STOPPED),
                            maximum_retry_count: None,
                        },
                    }),
                    mounts: if mounts.is_empty() {
                        None
                    } else {
                        Some(mounts)
                    },
                    log_config: Some(default_log_config()),
                    ..Default::default()
                }),
                networking_config: Some(NetworkingConfig {
                    endpoints_config: Some(endpoints_config),
                }),
                ..Default::default()
            },
        )
        .await?;

    tracing::info!(
        container = %container_name,
        app_id = %app.id,
        deployment_id = %deployment.id,
        process_type = %context.process_type,
        "container created"
    );

    Ok(())
}

/// Resolves and creates volume mounts for a container.
///
/// # Arguments
///
/// * `docker_client` - Docker API client ([`Docker`]).
/// * `app_id` - Application ID string.
/// * `volume_paths` - Vector of volume path strings.
///
/// # Returns
///
/// A [`DockerResult`] containing a vector of Docker volume mounts ([`Mount`]).
async fn build_mounts(
    docker_client: &Docker,
    app_id: &str,
    volume_paths: &[String],
) -> DockerResult<Vec<Mount>> {
    let mut paths: Vec<String> = vec![MANAGED_DATA_PATH.to_string()];
    for path in volume_paths {
        if path != MANAGED_DATA_PATH {
            paths.push(path.clone());
        }
    }

    let mut mounts = Vec::with_capacity(paths.len());

    for path in &paths {
        let volume_name = app_volume_name(app_id, path);
        let labels = HashMap::from([
            (
                crate::docker::labels::LABEL_APP_ID.to_string(),
                app_id.to_string(),
            ),
            ("path".to_string(), path.clone()),
        ]);

        utils::create_volume(docker_client, &volume_name, Some(labels)).await?;

        mounts.push(Mount {
            typ: Some(MountType::VOLUME),
            source: Some(volume_name),
            target: Some(path.clone()),
            ..Default::default()
        });
    }

    Ok(mounts)
}
