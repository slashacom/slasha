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

use crate::docker::{DockerError, DockerResult};

const RETAINED_IMAGES_PER_APP: usize = 10;

/// Generates the base Docker image name for an application.
///
/// # Arguments
///
/// * `app_slug` - Application slug string.
///
/// # Returns
///
/// Image repository name string.
pub fn base_image_name(app_slug: &str) -> String {
    format!("slasha/{}", app_slug)
}

/// Generates a tagged Docker image name for an application deployment.
///
/// # Arguments
///
/// * `app_slug` - Application slug string.
/// * `deployment_id` - Deployment ID string.
///
/// # Returns
///
/// Tagged image name string.
pub fn image_tag(app_slug: &str, deployment_id: &str) -> String {
    format!("slasha/{}:{}", app_slug, deployment_id)
}

/// Inspects the host Docker daemon to verify if a deployment image exists.
///
/// # Arguments
///
/// * `docker_client` - Docker API client ([`Docker`]).
/// * `app_slug` - Application slug string.
/// * `deployment_id` - Deployment ID string.
///
/// # Returns
///
/// A [`DockerResult`] containing the verified image tag string.
pub async fn find_deployment_image(
    docker_client: &Docker,
    app_slug: &str,
    deployment_id: &str,
) -> DockerResult<String> {
    let tag = image_tag(app_slug, deployment_id);
    if docker_client.inspect_image(&tag).await.is_ok() {
        return Ok(tag);
    }

    Err(DockerError::ArtifactUnavailable(deployment_id.to_string()))
}

/// Tags a source Docker image with a new target repository name and deployment tag.
///
/// # Arguments
///
/// * `docker_client` - Docker API client ([`Docker`]).
/// * `source` - Source image name.
/// * `app_slug` - Target application slug string.
/// * `deployment_id` - Target deployment ID string.
///
/// # Returns
///
/// A [`DockerResult`] containing the target image tag string.
pub async fn tag_deployment_image(
    docker_client: &Docker,
    source: &str,
    app_slug: &str,
    deployment_id: &str,
) -> DockerResult<String> {
    let target = image_tag(app_slug, deployment_id);
    let options = TagImageOptionsBuilder::new()
        .repo(base_image_name(app_slug).as_str())
        .tag(deployment_id)
        .build();

    docker_client.tag_image(source, Some(options)).await?;
    Ok(target)
}

/// Removes a specific deployment Docker image from the host daemon.
///
/// # Arguments
///
/// * `docker_client` - Docker API client ([`Docker`]).
/// * `app_slug` - Application slug string.
/// * `deployment_id` - Deployment ID string.
pub async fn remove_deployment_image(
    docker_client: &Docker,
    app_slug: &str,
    deployment_id: &str,
) -> DockerResult<()> {
    let tag = image_tag(app_slug, deployment_id);
    if docker_client.inspect_image(&tag).await.is_err() {
        return Ok(());
    }

    docker_client
        .remove_image(&tag, Some(RemoveImageOptions::default()), None)
        .await?;

    Ok(())
}

/// Force-deletes all built Docker images associated with an application slug.
///
/// # Arguments
///
/// * `docker_client` - Docker API client ([`Docker`]).
/// * `app_slug` - Application slug string.
pub async fn remove_app_images(docker_client: &Docker, app_slug: &str) -> DockerResult<()> {
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

/// Prunes older deployment images associated with an application, keeping recent ones.
///
/// # Arguments
///
/// * `docker_client` - Docker API client ([`Docker`]).
/// * `db_pool` - Database connection pool reference ([`DbPool`]).
/// * `app` - Target application model ([`App`]).
pub async fn prune_app_images(
    docker_client: &Docker,
    db_pool: &DbPool,
    app: &App,
) -> DockerResult<()> {
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

/// Lists all Docker image tags matching an application slug on the host daemon.
///
/// # Arguments
///
/// * `docker_client` - Docker API client ([`Docker`]).
/// * `app_slug` - Application slug string.
///
/// # Returns
///
/// A [`DockerResult`] containing a vector of image tag strings.
async fn list_app_image_tags(docker_client: &Docker, app_slug: &str) -> DockerResult<Vec<String>> {
    let mut filters = HashMap::new();
    filters.insert(
        "reference".to_string(),
        vec![format!("{}:*", base_image_name(app_slug))],
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
