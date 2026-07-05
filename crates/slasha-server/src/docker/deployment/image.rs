use std::collections::{HashMap, HashSet};

use bollard::{
    Docker,
    query_parameters::{ListImagesOptionsBuilder, RemoveImageOptions, TagImageOptionsBuilder},
};
use slasha_db::{
    DbPool,
    app::App,
    deployment::{Deployment, DeploymentStatus},
    repos::deployment::DeploymentRepo,
};

use crate::docker::{
    DeploymentError, DeploymentResult,
    naming::{image_name, image_tag},
};

const RETAINED_IMAGES_PER_APP: usize = 10;

pub async fn find_deployment_image(
    docker_client: &Docker,
    app: &App,
    deployment: &Deployment,
) -> DeploymentResult<String> {
    let tag = image_tag(&app.slug, &deployment.id);
    if docker_client.inspect_image(&tag).await.is_ok() {
        return Ok(tag);
    }

    Err(DeploymentError::ArtifactUnavailable(deployment.id.clone()))
}

pub async fn tag_deployment_image(
    docker_client: &Docker,
    source: &str,
    app_slug: &str,
    deployment_id: &str,
) -> DeploymentResult<String> {
    let target = image_tag(app_slug, deployment_id);
    let options = TagImageOptionsBuilder::new()
        .repo(image_name(app_slug).as_str())
        .tag(deployment_id)
        .build();
    docker_client.tag_image(source, Some(options)).await?;
    Ok(target)
}

pub async fn remove_deployment_image(
    docker_client: &Docker,
    app_slug: &str,
    deployment_id: &str,
) -> DeploymentResult<()> {
    let tag = image_tag(app_slug, deployment_id);
    if docker_client.inspect_image(&tag).await.is_err() {
        return Ok(());
    }

    docker_client
        .remove_image(&tag, Some(RemoveImageOptions::default()), None)
        .await?;
    Ok(())
}

pub async fn remove_app_images(docker_client: &Docker, app_slug: &str) -> DeploymentResult<()> {
    for tag in list_app_image_tags(docker_client, app_slug).await? {
        if let Err(error) = docker_client
            .remove_image(&tag, Some(RemoveImageOptions::default()), None)
            .await
        {
            tracing::warn!(image_tag = %tag, error = ?error, "Failed to remove app image");
        }
    }

    Ok(())
}

pub async fn prune_app_images(
    docker_client: &Docker,
    db_pool: &DbPool,
    app: &App,
) -> DeploymentResult<()> {
    let retained: Vec<Deployment> = DeploymentRepo::list_for_app(db_pool, &app.id)
        .await?
        .into_iter()
        .filter(|deployment| {
            matches!(
                deployment.status,
                DeploymentStatus::Running | DeploymentStatus::Stopped
            )
        })
        .take(RETAINED_IMAGES_PER_APP)
        .collect();

    let mut keep = HashSet::new();
    for deployment in retained {
        keep.insert(image_tag(&app.slug, &deployment.id));
    }

    for tag in list_app_image_tags(docker_client, &app.slug).await? {
        if keep.contains(&tag) {
            continue;
        }

        if let Err(error) = docker_client
            .remove_image(&tag, Some(RemoveImageOptions::default()), None)
            .await
        {
            tracing::warn!(image_tag = %tag, error = ?error, "Failed to prune deployment image");
        } else {
            tracing::info!(image_tag = %tag, "Deployment image pruned");
        }
    }

    Ok(())
}

async fn list_app_image_tags(
    docker_client: &Docker,
    app_slug: &str,
) -> DeploymentResult<Vec<String>> {
    let mut filters = HashMap::new();
    filters.insert(
        "reference".to_string(),
        vec![format!("{}:*", image_name(app_slug))],
    );
    let options = ListImagesOptionsBuilder::new()
        .all(true)
        .filters(&filters)
        .build();

    Ok(docker_client
        .list_images(Some(options))
        .await?
        .into_iter()
        .flat_map(|image| image.repo_tags)
        .collect())
}
