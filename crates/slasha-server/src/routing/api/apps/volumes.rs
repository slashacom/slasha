use std::{collections::HashMap, path::Path};

use axum::{Json, Router, response::IntoResponse, routing::get};
use bollard::{Docker, query_parameters::DataUsageOptions};
use serde::Serialize;

use crate::{
    AppState, HttpResult,
    docker::{
        deployment::{
            BuildStrategy, container::MANAGED_DATA_PATH, detect_build_strategy, parse_volumes,
            resolve_head_commit,
        },
        naming::app_volume_name,
    },
    extractors::app::ActiveApp,
};

pub fn router() -> Router<AppState> {
    Router::new().route("/", get(list_volumes))
}

#[derive(Serialize)]
struct VolumeView {
    path: String,
    managed: bool,
    exists: bool,
    size_bytes: Option<i64>,
}

async fn dockerfile_volume_paths(repo_path: &str, branch: &str) -> Vec<String> {
    let Ok((commit_sha, _)) = resolve_head_commit(repo_path, branch) else {
        return Vec::new();
    };
    let path = Path::new(repo_path);
    let Ok(strategy) = detect_build_strategy(path, &commit_sha).await else {
        return Vec::new();
    };
    match strategy {
        BuildStrategy::Dockerfile { content } => parse_volumes(&content),
        BuildStrategy::Railpack => Vec::new(),
    }
}

async fn volume_sizes(docker: &Docker) -> HashMap<String, i64> {
    let mut sizes = HashMap::new();
    let Ok(usage) = docker.df(None::<DataUsageOptions>).await else {
        return sizes;
    };
    let items = usage.volume_usage.and_then(|v| v.items).unwrap_or_default();
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
    ActiveApp {
        app, docker_client, ..
    }: ActiveApp,
) -> HttpResult<impl IntoResponse> {
    let mut paths = vec![MANAGED_DATA_PATH.to_string()];
    for path in dockerfile_volume_paths(&app.repo_path, &app.default_branch).await {
        if path != MANAGED_DATA_PATH && !paths.contains(&path) {
            paths.push(path);
        }
    }

    let sizes = volume_sizes(&docker_client).await;

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
