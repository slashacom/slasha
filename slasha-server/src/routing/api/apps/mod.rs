pub mod deployments;
pub mod env;
pub mod files;
pub mod management;
pub mod service_env;
pub mod services;
mod utils;

use crate::{
    AppState,
    error::{Error, Result},
    extractors::auth::AuthUser,
    state::Storage,
};
use axum::{
    Json, Router,
    extract::{Path, State},
    response::IntoResponse,
    routing::get,
};

#[derive(serde::Serialize)]
struct CommitInfo {
    sha: String,
    message: String,
}

fn get_all_commits(repo_path: &str, branch_name: &str) -> anyhow::Result<Vec<CommitInfo>> {
    let repo = git2::Repository::open(repo_path)?;
    let branch = repo.find_branch(branch_name, git2::BranchType::Local)?;
    let commit = branch.get().peel_to_commit()?;
    tracing::info!("Commit id: {}", commit.id());
    tracing::info!("repo path: {}", repo_path);
    tracing::info!("branch name: {}", branch_name);
    tracing::info!("branch: {}", branch.name().unwrap().unwrap());

    let mut revwalk = repo.revwalk()?;
    revwalk.push(commit.id())?;

    let mut commits = Vec::new();

    for commit_id in revwalk {
        tracing::info!("Commit ID: {:?}", commit_id);
        let commit = repo.find_commit(commit_id?)?;
        commits.push(CommitInfo {
            sha: commit.id().to_string(),
            message: commit.summary().unwrap_or("").to_string(),
        });
    }

    Ok(commits)
}

async fn list_commits(
    State(storage): State<Storage>,
    AuthUser(user): AuthUser,
    Path(slug): Path<String>,
) -> Result<impl IntoResponse> {
    let app = utils::lookup_app_for_user(&storage, &slug, &user.id)?;

    let commits = get_all_commits(&app.repo_path, &app.default_branch)
        .map_err(|e| Error::Internal(anyhow::anyhow!("Failed to fetch commits: {}", e)))?;

    Ok(Json(serde_json::json!({ "commits": commits })))
}

pub fn router() -> Router<AppState> {
    Router::new()
        .merge(management::router())
        .nest("/{slug}/env", env::router())
        .nest("/{slug}/files", files::router())
        .route("/{slug}/commits", get(list_commits))
        .nest("/{slug}/deployments", deployments::router())
        .nest("/{slug}/services", services::router())
        .nest("/{slug}/services/{service_id}/env", service_env::router())
}
