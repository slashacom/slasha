use std::{collections::HashMap, path::Path};

use axum::{
    Json, Router,
    extract::{Path as AxumPath, State},
    response::IntoResponse,
    routing::get,
};
use bollard::{Docker, query_parameters::DataUsageOptions};
use serde::Serialize;
use slasha_db::repos::app::AppRepo;

use crate::{
    AppState,
    docker::{
        deployment::{container::MANAGED_DATA_PATH, parse_volumes},
        naming::app_volume_name,
    },
    error::HttpResult,
    extractors::auth::AuthUser,
    state::Storage,
};

pub fn router() -> Router<AppState> {
    Router::new().route("/", get(list_volumes))
}

#[derive(Serialize)]
struct VolumeView {
    path: String,
    /// True for the per-app volume slasha mounts automatically at every deploy.
    managed: bool,
    /// Whether the underlying docker volume has been created yet (i.e. the app
    /// has deployed at least once).
    exists: bool,
    /// On-disk size in bytes, when reported by the docker daemon.
    size_bytes: Option<i64>,
}

/// VOLUME paths declared in the repo's Dockerfile on its default branch.
fn dockerfile_volume_paths(repo_path: &str, branch: &str) -> Vec<String> {
    (|| -> anyhow::Result<Vec<String>> {
        let repo = git2::Repository::open(repo_path)?;
        let branch = repo.find_branch(branch, git2::BranchType::Local)?;
        let tree = branch.get().peel_to_tree()?;
        let entry = tree.get_path(Path::new("Dockerfile"))?;
        let blob = repo.find_blob(entry.id())?;
        let content = std::str::from_utf8(blob.content())?.to_string();
        Ok(parse_volumes(&content))
    })()
    .unwrap_or_default()
}

/// Map of docker volume name -> on-disk size in bytes (negative when the daemon
/// can't report it).
async fn volume_sizes(docker: &Docker) -> HashMap<String, i64> {
    let mut sizes = HashMap::new();
    let Ok(usage) = docker.df(None::<DataUsageOptions>).await else {
        return sizes;
    };
    let items = usage
        .volumes_disk_usage
        .and_then(|v| v.items)
        .unwrap_or_default();
    for item in items {
        let Some(name) = item.get("Name").and_then(|v| v.as_str()) else {
            continue;
        };
        let size = item
            .pointer("/UsageData/Size")
            .and_then(|v| v.as_i64())
            .unwrap_or(-1);
        sizes.insert(name.to_string(), size);
    }
    sizes
}

async fn list_volumes(
    State(storage): State<Storage>,
    State(docker): State<Docker>,
    AuthUser(user): AuthUser,
    AxumPath(slug): AxumPath<String>,
) -> HttpResult<impl IntoResponse> {
    let app = AppRepo::find_by_slug_for_user(&storage.db_pool, &slug, &user.id).await?;

    let mut paths = vec![MANAGED_DATA_PATH.to_string()];
    for path in dockerfile_volume_paths(&app.repo_path, &app.default_branch) {
        if path != MANAGED_DATA_PATH && !paths.contains(&path) {
            paths.push(path);
        }
    }

    let sizes = volume_sizes(&docker).await;

    let volumes: Vec<VolumeView> = paths
        .into_iter()
        .map(|path| {
            let name = app_volume_name(&app.id, &path);
            let reported = sizes.get(&name).copied();
            VolumeView {
                managed: path == MANAGED_DATA_PATH,
                exists: reported.is_some(),
                size_bytes: reported.filter(|s| *s >= 0),
                path,
            }
        })
        .collect();

    Ok(Json(serde_json::json!({ "volumes": volumes })))
}
