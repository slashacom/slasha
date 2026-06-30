use axum::{
    body::Bytes,
    extract::State,
    http::{HeaderMap, StatusCode},
};
use hmac::{Hmac, KeyInit, Mac};
use serde::Deserialize;
use sha2::Sha256;
use slasha_db::{
    app::AppSource,
    github_connection::ConnectionStatus,
    repos::{app::AppRepo, deployment::DeploymentRepo, github_connection::GithubConnectionRepo},
};

use crate::{
    AppState,
    connections::sync_github_app,
    docker::deployment::trigger_deployment,
    error::{HttpError, HttpResult},
};

type HmacSha256 = Hmac<Sha256>;

#[derive(Deserialize)]
struct InstallationRef {
    id: i64,
}

#[derive(Deserialize)]
struct RepositoryRef {
    id: i64,
}

#[derive(Deserialize)]
struct PushPayload {
    installation: InstallationRef,
    repository: RepositoryRef,
    r#ref: String,
    after: String,
    deleted: bool,
}

#[derive(Deserialize)]
struct InstallationPayload {
    action: String,
    installation: InstallationRef,
}

#[derive(Deserialize)]
struct InstallationRepositoriesPayload {
    action: String,
    installation: InstallationRef,
    repositories_removed: Vec<RepositoryRef>,
}

pub async fn handle(
    State(state): State<AppState>,
    headers: HeaderMap,
    body: Bytes,
) -> HttpResult<StatusCode> {
    verify_signature(&state, &headers, &body).await?;

    let event = headers
        .get("x-github-event")
        .and_then(|value| value.to_str().ok())
        .ok_or_else(|| HttpError::bad_request("Missing GitHub event header"))?;

    match event {
        "push" => {
            let payload: PushPayload =
                serde_json::from_slice(&body).map_err(HttpError::internal)?;
            if !payload.deleted {
                tokio::spawn(handle_push(state, payload));
            }
        }
        "installation" => {
            let payload: InstallationPayload =
                serde_json::from_slice(&body).map_err(HttpError::internal)?;
            if matches!(payload.action.as_str(), "deleted" | "suspend") {
                disconnect_installation(&state, payload.installation.id).await?;
            }
        }
        "installation_repositories" => {
            let payload: InstallationRepositoriesPayload =
                serde_json::from_slice(&body).map_err(HttpError::internal)?;
            if payload.action == "removed" {
                for repository in payload.repositories_removed {
                    disconnect_repository(&state, payload.installation.id, repository.id).await?;
                }
            }
        }
        _ => {}
    }

    Ok(StatusCode::ACCEPTED)
}

async fn verify_signature(state: &AppState, headers: &HeaderMap, body: &[u8]) -> HttpResult<()> {
    let github = state
        .github_client()
        .await
        .ok_or_else(|| HttpError::not_found("GitHub integration is disabled"))?;
    let signature = headers
        .get("x-hub-signature-256")
        .and_then(|value| value.to_str().ok())
        .ok_or_else(HttpError::unauthorized)?;
    let Some(signature) = signature.strip_prefix("sha256=") else {
        return Err(HttpError::unauthorized());
    };
    let signature = hex::decode(signature).map_err(|_| HttpError::unauthorized())?;
    let mut mac =
        HmacSha256::new_from_slice(github.webhook_secret()).map_err(HttpError::internal)?;
    mac.update(body);
    mac.verify_slice(&signature)
        .map_err(|_| HttpError::unauthorized())
}

async fn disconnect_installation(state: &AppState, installation_id: i64) -> HttpResult<()> {
    GithubConnectionRepo::disconnect_installation(&state.storage.db_pool, installation_id).await?;
    Ok(())
}

async fn disconnect_repository(
    state: &AppState,
    installation_id: i64,
    repository_id: i64,
) -> HttpResult<()> {
    let connections = GithubConnectionRepo::list_for_repository(
        &state.storage.db_pool,
        installation_id,
        repository_id,
    )
    .await?;
    for connection in connections {
        GithubConnectionRepo::update_status(
            &state.storage.db_pool,
            &connection.app_id,
            ConnectionStatus::Disconnected,
        )
        .await?;
    }
    Ok(())
}

async fn handle_push(state: AppState, payload: PushPayload) {
    let Some(github) = state.github_client().await else {
        return;
    };
    let connections = match GithubConnectionRepo::list_for_repository(
        &state.storage.db_pool,
        payload.installation.id,
        payload.repository.id,
    )
    .await
    {
        Ok(connections) => connections,
        Err(error) => {
            tracing::warn!(error = %error, "github push lookup failed");
            return;
        }
    };

    for connection in connections {
        if connection.status != ConnectionStatus::Connected {
            continue;
        }
        let mut app = match AppRepo::find_by_id(&state.storage.db_pool, &connection.app_id).await {
            Ok(app) if app.source == AppSource::Github => app,
            _ => continue,
        };
        let repository = match sync_github_app(&github, &state.storage, &state.runtime, &app).await
        {
            Ok(repository) => repository,
            Err(error) => {
                tracing::warn!(app_id = %app.id, error = %error, "github repository sync failed");
                continue;
            }
        };
        if payload.r#ref != format!("refs/heads/{}", repository.default_branch) {
            continue;
        }
        if app.default_branch != repository.default_branch {
            if let Err(error) = AppRepo::update_default_branch(
                &state.storage.db_pool,
                &app.id,
                &repository.default_branch,
            )
            .await
            {
                tracing::warn!(
                    app_id = %app.id,
                    error = %error,
                    "failed to update github default branch"
                );
                continue;
            }
            app.default_branch = repository.default_branch;
        }
        if !app.auto_deploy {
            continue;
        }
        if let Ok(deployments) = DeploymentRepo::list_for_app(&state.storage.db_pool, &app.id).await
            && deployments
                .first()
                .map(|deployment| deployment.commit_sha.as_str())
                == Some(payload.after.as_str())
        {
            continue;
        }

        match trigger_deployment(
            state.clients.docker.clone(),
            state.storage.db_pool.clone(),
            state.runtime.log_manager.clone(),
            state.runtime.proxy_sync_trigger.clone(),
            app,
            Some(payload.after.clone()),
        )
        .await
        {
            Ok(Some(deployment)) => tracing::info!(
                deployment_id = %deployment.id,
                "auto-deploy triggered from github push"
            ),
            Ok(None) => tracing::info!("github auto-deploy skipped: build already in progress"),
            Err(error) => tracing::warn!(error = %error, "github auto-deploy failed"),
        }
    }
}
