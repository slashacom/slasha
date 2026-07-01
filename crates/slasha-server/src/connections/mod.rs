pub mod github;
mod mirror;

use std::path::PathBuf;

pub use github::{
    GithubClient, GithubError, GithubInstallationInfo, GithubRepository, GithubResult,
    create_state, handle_webhook, verify_state,
};
use slasha_db::{
    app::{App, AppSource},
    github_connection::ConnectionStatus,
    repos::{git_connection::GitConnectionRepo, github_connection::GithubConnectionRepo},
};

use crate::state::{Runtime, Storage};

pub async fn sync_external_app(
    github: Option<&GithubClient>,
    storage: &Storage,
    runtime: &Runtime,
    app: &mut App,
) -> anyhow::Result<()> {
    match app.source {
        AppSource::Github => {
            sync_github_app(
                github.ok_or_else(|| anyhow::anyhow!("GitHub integration is disabled"))?,
                storage,
                runtime,
                app,
            )
            .await?;
        }
        AppSource::Git => {
            sync_git_app(storage, runtime, app).await?;
        }
        _ => return Ok(()),
    };

    Ok(())
}

pub async fn sync_github_app(
    client: &GithubClient,
    storage: &Storage,
    runtime: &Runtime,
    app: &App,
) -> anyhow::Result<()> {
    let connection = GithubConnectionRepo::find_for_app(&storage.db_pool, &app.id)
        .await?
        .ok_or_else(|| anyhow::anyhow!("GitHub connection not found"))?;
    if connection.status != ConnectionStatus::Connected {
        anyhow::bail!("GitHub connection is disconnected");
    }

    let lock = runtime.get_connection_sync_lock(&app.id);
    let _guard = lock.lock().await;

    let (repository, token) = match client
        .get_repository_with_token(connection.installation_id, connection.repository_id)
        .await
    {
        Ok(repository) => repository,
        Err(GithubError::AccessRevoked) => {
            GithubConnectionRepo::update_status(
                &storage.db_pool,
                &app.id,
                ConnectionStatus::Disconnected,
            )
            .await?;
            return Err(GithubError::AccessRevoked.into());
        }
        Err(error) => return Err(error.into()),
    };

    mirror::Mirror {
        remote_url: repository.clone_url.clone(),
        branch: Some(app.default_branch.clone()),
        path: PathBuf::from(&app.repo_path),
        auth: mirror::MirrorAuth::GithubToken(token),
    }
    .sync()
    .await?;

    Ok(())
}

pub async fn sync_selected_github_repository(
    client: &GithubClient,
    runtime: &Runtime,
    app_id: &str,
    repo_path: PathBuf,
    installation_id: i64,
    repository_id: i64,
    branch: Option<String>,
) -> anyhow::Result<GithubRepository> {
    // we need to acquire the lock since we also call this when reconnecting
    let lock = runtime.get_connection_sync_lock(app_id);
    let _guard = lock.lock().await;

    let (repository, token) = client
        .get_repository_with_token(installation_id, repository_id)
        .await?;

    let branch = branch.or_else(|| Some(repository.default_branch.clone()));

    mirror::Mirror {
        remote_url: repository.clone_url.clone(),
        branch,
        path: repo_path,
        auth: mirror::MirrorAuth::GithubToken(token),
    }
    .sync()
    .await?;

    Ok(repository)
}

async fn sync_git_app(storage: &Storage, runtime: &Runtime, app: &App) -> anyhow::Result<String> {
    let connection = GitConnectionRepo::find_for_app(&storage.db_pool, &app.id)
        .await?
        .ok_or_else(|| anyhow::anyhow!("Git connection not found"))?;

    let lock = runtime.get_connection_sync_lock(&app.id);
    let _guard = lock.lock().await;

    sync_selected_git_repository(
        connection.clone_url,
        Some(app.default_branch.clone()),
        PathBuf::from(&app.repo_path),
    )
    .await
}

pub async fn sync_selected_git_repository(
    clone_url: String,
    branch: Option<String>,
    repo_path: PathBuf,
) -> anyhow::Result<String> {
    mirror::Mirror {
        remote_url: clone_url,
        branch,
        path: repo_path,
        auth: mirror::MirrorAuth::Anonymous,
    }
    .sync()
    .await
}
