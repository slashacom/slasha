use std::collections::HashMap;

use bollard::{
    Docker, body_try_stream,
    models::{ContainerCreateBody, HostConfig, Mount, MountType, VolumeCreateRequest},
    query_parameters::{
        CreateContainerOptions, CreateImageOptions, DownloadFromContainerOptions,
        ListVolumesOptions, RemoveContainerOptionsBuilder, RemoveVolumeOptions,
        UploadToContainerOptions,
    },
};
use futures_util::{StreamExt, TryStreamExt};
use slasha_db::{
    DbPool,
    app::App,
    deployment::Deployment,
    models::node::Node,
    repos::{app::AppRepo, deployment::DeploymentRepo, service::ServiceRepo},
};

use crate::{
    docker::{
        DeploymentError, DeploymentResult,
        deployment::{
            purge_app_from_node, restart_deployment_processes, stop_deployment_processes,
            trigger_deployment,
        },
        naming::{app_volume_prefix, service_volume_name},
        network::{create_app_network, remove_app_network},
        rollback::Rollback,
        service::provision_service,
    },
    state::Runtime,
};

const HELPER_IMAGE: &str = "alpine:latest";

async fn ensure_alpine_image(docker: &Docker) -> DeploymentResult<()> {
    let mut stream = docker.create_image(
        Some(CreateImageOptions {
            from_image: Some(HELPER_IMAGE.to_string()),
            ..Default::default()
        }),
        None,
        None,
    );

    while let Some(item) = stream.next().await {
        item.map_err(DeploymentError::DockerApi)?;
    }

    Ok(())
}

async fn create_helper_container(
    docker_client: &Docker,
    container_name: &str,
    volume_name: &str,
) -> DeploymentResult<()> {
    let body = ContainerCreateBody {
        image: Some(HELPER_IMAGE.to_string()),
        host_config: Some(HostConfig {
            mounts: Some(vec![Mount {
                typ: Some(MountType::VOLUME),
                source: Some(volume_name.to_string()),
                target: Some("/volume_data".to_string()),
                ..Default::default()
            }]),
            ..Default::default()
        }),
        ..Default::default()
    };

    docker_client
        .create_container(
            Some(CreateContainerOptions {
                name: Some(container_name.to_string()),
                ..Default::default()
            }),
            body,
        )
        .await?;
    Ok(())
}

async fn migrate_single_volume(
    old_docker_client: &Docker,
    new_docker_client: &Docker,
    volume_name: &str,
) -> DeploymentResult<()> {
    let helper_src_name = format!("slasha-move-src-{}", uuid::Uuid::new_v4());
    let helper_dst_name = format!("slasha-move-dst-{}", uuid::Uuid::new_v4());

    new_docker_client
        .create_volume(VolumeCreateRequest {
            name: Some(volume_name.to_string()),
            ..Default::default()
        })
        .await?;

    create_helper_container(old_docker_client, &helper_src_name, volume_name).await?;
    create_helper_container(new_docker_client, &helper_dst_name, volume_name).await?;

    let stream = old_docker_client
        .download_from_container(
            &helper_src_name,
            Some(DownloadFromContainerOptions {
                path: "/volume_data".to_string(),
            }),
        )
        .map_err(std::io::Error::other);

    let body = body_try_stream(stream);
    new_docker_client
        .upload_to_container(
            &helper_dst_name,
            Some(UploadToContainerOptions {
                path: "/".to_string(),
                no_overwrite_dir_non_dir: Some("false".to_string()),
                ..Default::default()
            }),
            body,
        )
        .await?;

    let remove_options = Some(RemoveContainerOptionsBuilder::new().force(true).build());
    let _ = old_docker_client
        .remove_container(&helper_src_name, remove_options.clone())
        .await;
    let _ = new_docker_client
        .remove_container(&helper_dst_name, remove_options)
        .await;

    Ok(())
}

pub async fn move_app_to_node(
    old_docker_client: &Docker,
    new_docker_client: &Docker,
    db_pool: &DbPool,
    runtime: &Runtime,
    app: &App,
    new_node: &Node,
) -> DeploymentResult<Option<Deployment>> {
    ensure_alpine_image(old_docker_client).await?;
    ensure_alpine_image(new_docker_client).await?;

    // this gives us deployments currently building or running
    let active_deployments = DeploymentRepo::list_active_for_app(db_pool, &app.id).await?;
    let was_running = !active_deployments.is_empty();

    for dep in &active_deployments {
        stop_deployment_processes(
            old_docker_client,
            db_pool,
            &runtime.proxy_sync_trigger,
            &runtime.log_manager,
            &runtime.deployment_tasks,
            app,
            dep,
        )
        .await?; // this will cancel builds and stop running containers
    }

    let mut rollback = Rollback::new();

    if was_running {
        rollback.register({
            let old_docker_client = old_docker_client.clone();
            let log_manager = runtime.log_manager.clone();
            let proxy_sync = runtime.proxy_sync_trigger.clone();
            let app = app.clone();
            let active_deployments = active_deployments.clone();

            move || {
                Box::pin(async move {
                    for dep in &active_deployments {
                        let _ = restart_deployment_processes(
                            &old_docker_client,
                            &log_manager,
                            &proxy_sync,
                            &app,
                            &dep.id,
                        )
                        .await;
                    }
                })
            }
        });
    }

    let migrate_logic = async {
        // migrate app volumes
        let prefix = app_volume_prefix(&app.id);
        let mut list_filters = HashMap::new();
        list_filters.insert("name".to_string(), vec![prefix.clone()]);
        let response = old_docker_client
            .list_volumes(Some(ListVolumesOptions {
                filters: Some(list_filters),
            }))
            .await?;

        let volumes: Vec<String> = response
            .volumes
            .unwrap_or_default()
            .into_iter()
            .map(|v| v.name)
            .filter(|n| n.starts_with(&prefix))
            .collect();

        for volume_name in volumes {
            migrate_single_volume(old_docker_client, new_docker_client, &volume_name).await?;

            rollback.register({
                let new_docker_client = new_docker_client.clone();
                let volume_name = volume_name.clone();
                move || {
                    Box::pin(async move {
                        let _ = new_docker_client
                            .remove_volume(&volume_name, None::<RemoveVolumeOptions>)
                            .await;
                    })
                }
            });
        }

        // migrate service volumes
        let app_services = ServiceRepo::list_for_app(db_pool, &app.id).await?;
        let mut service_volumes = Vec::new();
        for service in &app_services {
            let volume_name = service_volume_name(&service.id);

            migrate_single_volume(old_docker_client, new_docker_client, &volume_name).await?;

            rollback.register({
                let new_docker_client = new_docker_client.clone();
                let volume_name = volume_name.clone();
                move || {
                    Box::pin(async move {
                        let _ = new_docker_client
                            .remove_volume(&volume_name, None::<RemoveVolumeOptions>)
                            .await;
                    })
                }
            });

            service_volumes.push(volume_name);
        }

        AppRepo::update_node(db_pool, &app.id, &new_node.id).await?;
        rollback.register({
            let db_pool = db_pool.clone();
            let app_id = app.id.clone();
            let old_node_id = app.node_id.clone();
            move || {
                Box::pin(async move {
                    let _ = AppRepo::update_node(&db_pool, &app_id, &old_node_id).await;
                })
            }
        });

        create_app_network(new_docker_client, &app.id).await?;
        rollback.register({
            let new_docker_client = new_docker_client.clone();
            let app_id = app.id.clone();
            move || {
                Box::pin(async move {
                    let _ = remove_app_network(&new_docker_client, &app_id).await;
                })
            }
        });

        // re-provision app services on the new node
        let mut new_app = app.clone();
        new_app.node_id = new_node.id.clone();

        for service in app_services {
            provision_service(
                new_docker_client.clone(),
                db_pool.clone(),
                runtime.log_manager.clone(),
                new_app.clone(),
                service,
                None,
            )
            .await?;
        }

        let new_deployment = if was_running {
            trigger_deployment(
                new_docker_client.clone(),
                db_pool.clone(),
                runtime.log_manager.clone(),
                runtime.proxy_sync_trigger.clone(),
                runtime.deployment_tasks.clone(),
                new_app,
                None,
            )
            .await?
        } else {
            None
        };

        Ok(new_deployment)
    };

    let result = migrate_logic.await;

    match result {
        Ok(new_deployment) => {
            rollback.disarm();

            tokio::spawn({
                let old_docker_client = old_docker_client.clone();
                let proxy_sync = runtime.proxy_sync_trigger.clone();
                let log_manager = runtime.log_manager.clone();
                let app = app.clone();
                let db_pool = db_pool.clone();

                async move {
                    if let Err(e) = purge_app_from_node(
                        &old_docker_client,
                        &db_pool,
                        &log_manager,
                        &proxy_sync,
                        &app,
                    )
                    .await
                    {
                        tracing::warn!(app_id = %app.id, error = ?e, "Failed to purge app from old node");
                    }
                }
            });

            Ok(new_deployment)
        }
        Err(e) => {
            rollback.execute().await;
            Err(e)
        }
    }
}
