use axum::{Json, Router, extract::Query, routing::get};
use serde::{Deserialize, Serialize};
use tokio::process::Command;

use crate::{
    AppState,
    error::{HttpError, HttpResult},
};

#[derive(Deserialize)]
pub struct GitRemoteQuery {
    pub url: String,
}

#[derive(Serialize)]
pub struct GitRemoteResponse {
    pub default_branch: Option<String>,
    pub branches: Vec<String>,
}

pub fn router() -> Router<AppState> {
    Router::new().route("/remote-branches", get(get_remote_branches))
}

async fn get_remote_branches(
    Query(query): Query<GitRemoteQuery>,
) -> HttpResult<Json<GitRemoteResponse>> {
    let url = query.url.trim();
    if url.is_empty() {
        return Err(HttpError::bad_request("URL is required"));
    }

    let output = Command::new("git")
        .arg("ls-remote")
        .arg("--symref")
        .arg(url)
        .arg("HEAD")
        .arg("refs/heads/*")
        .output()
        .await
        .map_err(|e| HttpError::internal(anyhow::anyhow!("Failed to run git ls-remote: {}", e)))?;

    if !output.status.success() {
        let err = String::from_utf8_lossy(&output.stderr);
        tracing::warn!(url = %url, "git ls-remote failed: {}", err);
        return Err(HttpError::bad_request(
            "Failed to fetch repository metadata. Ensure the repository is public and the URL is correct.",
        ));
    }

    let stdout = String::from_utf8_lossy(&output.stdout);
    let mut default_branch = None;
    let mut branches = Vec::new();

    for line in stdout.lines() {
        let parts: Vec<&str> = line.split_whitespace().collect();
        if parts.is_empty() {
            continue;
        }

        // output format:
        // ref: refs/heads/main  HEAD
        // 3e8... HEAD
        // 3e8... refs/heads/main
        // 4a2... refs/heads/dev

        if parts[0] == "ref:" {
            if parts.len() >= 3
                && parts[2] == "HEAD"
                && let Some(branch) = parts[1].strip_prefix("refs/heads/")
            {
                default_branch = Some(branch.to_string());
            }
        } else if parts.len() >= 2 {
            let ref_name = parts[1];
            if let Some(branch) = ref_name.strip_prefix("refs/heads/") {
                branches.push(branch.to_string());
            }
        }
    }

    Ok(Json(GitRemoteResponse {
        default_branch,
        branches,
    }))
}
