mod deployments;
mod domains;
mod env;
mod files;
mod management;
mod metrics;
mod service_env;
mod services;

use axum::{
    Json, Router,
    extract::{Path, State},
    response::IntoResponse,
    routing::get,
};
use slasha_db::repos::app::AppRepo;

use crate::{
    AppState,
    error::{HttpError, HttpResult},
    extractors::auth::AuthUser,
    state::Storage,
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

    let mut revwalk = repo.revwalk()?;
    revwalk.push(commit.id())?;

    let mut commits = Vec::new();

    for commit_id in revwalk {
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
) -> HttpResult<impl IntoResponse> {
    let app = AppRepo::find_by_slug_for_user(&storage.db_pool, &slug, &user.id).await?;

    let commits = get_all_commits(&app.repo_path, &app.default_branch)
        .map_err(|e| HttpError::internal(anyhow::anyhow!("Failed to fetch commits: {}", e)))?;

    Ok(Json(serde_json::json!({ "commits": commits })))
}

pub fn router() -> Router<AppState> {
    Router::new()
        .merge(management::router())
        .nest("/{slug}/env", env::router())
        .nest("/{slug}/files", files::router())
        .route("/{slug}/commits", get(list_commits))
        .nest("/{slug}/deployments", deployments::router())
        .nest("/{slug}/domains", domains::router())
        .nest("/{slug}/services", services::router())
        .route("/{slug}/metrics", get(metrics::get_metrics))
        .nest("/{slug}/services/{service_id}/env", service_env::router())
}
