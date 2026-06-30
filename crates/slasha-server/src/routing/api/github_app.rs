use axum::{
    Json, Router,
    extract::{Path, Query, State},
    http::StatusCode,
    response::{IntoResponse, Redirect},
    routing::{delete, get, post},
};
use chrono::Utc;
use serde::Deserialize;
use slasha_db::{
    github_connection::GithubInstallation,
    repos::{github_connection::GithubConnectionRepo, user::UserRepo},
};

use crate::{
    AppState,
    connections::{GithubError, create_state, handle_webhook, verify_state},
    error::{HttpError, HttpResult},
    extractors::auth::AuthUser,
};

pub fn router() -> Router<AppState> {
    Router::new()
        .route("/status", get(status))
        .route("/install", post(install))
        .route("/callback", get(callback))
        .route("/repositories", get(repositories))
        .route(
            "/installations/{installation_id}",
            delete(remove_installation),
        )
        .route("/webhook", post(handle_webhook))
}

async fn get_github_client(state: &AppState) -> HttpResult<crate::connections::GithubClient> {
    state
        .github_client()
        .await
        .ok_or_else(|| HttpError::not_found("GitHub integration is disabled"))
}

async fn status(State(state): State<AppState>) -> Json<serde_json::Value> {
    Json(serde_json::json!({
        "enabled": state.github_client().await.is_some(),
    }))
}

#[derive(Deserialize)]
struct InstallReq {
    redirect_to: String,
}

async fn install(
    State(state): State<AppState>,
    AuthUser(user): AuthUser,
    Json(payload): Json<InstallReq>,
) -> HttpResult<impl IntoResponse> {
    let github_client = get_github_client(&state).await?;
    let redirect_to = validate_redirect_target(&payload.redirect_to)?;
    let state_token = create_state(&user.id, &redirect_to, &state.config.jwt_secret)?;
    let url = github_client
        .get_installation_url(&state_token)
        .await
        .map_err(|e| HttpError::internal(anyhow::Error::from(e)))?;
    Ok(Json(serde_json::json!({ "url": url })))
}

fn validate_redirect_target(target: &str) -> HttpResult<String> {
    if !target.starts_with('/') || target.starts_with("//") {
        return Err(HttpError::bad_request(
            "GitHub redirect must be an application-relative path",
        ));
    }
    Ok(target.to_string())
}

#[derive(Deserialize)]
struct CallbackQuery {
    code: String,
    state: String,
    installation_id: i64,
}

async fn callback(
    State(state): State<AppState>,
    Query(query): Query<CallbackQuery>,
) -> HttpResult<Redirect> {
    let github_client = get_github_client(&state).await?;
    let (user_id, redirect_to) = verify_state(&query.state, &state.config.jwt_secret)
        .map_err(|_| HttpError::bad_request("Invalid or expired GitHub state"))?;
    UserRepo::find_by_id(&state.storage.db_pool, &user_id)
        .await
        .map_err(|_| HttpError::unauthorized())?;

    let user_token = github_client
        .exchange_oauth_code(&query.code)
        .await
        .map_err(|e| HttpError::internal(anyhow::Error::from(e)))?;

    if !github_client
        .user_has_installation_access(&user_token, query.installation_id)
        .await
        .map_err(|e| HttpError::internal(anyhow::Error::from(e)))?
    {
        return Err(HttpError::forbidden(
            "GitHub installation is not accessible to this user",
        ));
    }

    GithubConnectionRepo::save_installation(
        &state.storage.db_pool,
        GithubInstallation {
            user_id: user_id.clone(),
            installation_id: query.installation_id,
            created_at: Utc::now().naive_utc(),
        },
    )
    .await?;

    Ok(Redirect::to(&redirect_to))
}

async fn repositories(
    State(state): State<AppState>,
    AuthUser(user): AuthUser,
) -> HttpResult<impl IntoResponse> {
    let github_client = get_github_client(&state).await?;
    let db_installations =
        GithubConnectionRepo::list_installations_for_user(&state.storage.db_pool, &user.id).await?;

    let mut installations = Vec::new();
    let mut all_repositories = Vec::new();

    for db_inst in db_installations {
        let installation_id = db_inst.installation_id;
        let gh_inst = match github_client.get_installation(installation_id).await {
            Ok(inst) => inst,
            Err(GithubError::AccessRevoked) => {
                GithubConnectionRepo::disconnect_installation(
                    &state.storage.db_pool,
                    installation_id,
                )
                .await?;
                continue;
            }
            Err(error) => return Err(HttpError::internal(anyhow::Error::from(error))),
        };

        let repositories = match github_client.get_repositories(installation_id).await {
            Ok(repos) => repos,
            Err(GithubError::AccessRevoked) => {
                GithubConnectionRepo::disconnect_installation(
                    &state.storage.db_pool,
                    installation_id,
                )
                .await?;
                continue;
            }
            Err(error) => return Err(HttpError::internal(anyhow::Error::from(error))),
        };

        installations.push(serde_json::json!({
            "installation_id": gh_inst.id,
            "configure_url": gh_inst.html_url,
            "repositories_count": repositories.len(),
        }));

        for repo in repositories {
            all_repositories.push(serde_json::json!({
                "id": repo.id,
                "full_name": repo.full_name,
                "default_branch": repo.default_branch,
                "private": repo.private,
                "installation_id": installation_id,
            }));
        }
    }

    Ok(Json(serde_json::json!({
        "installations": installations,
        "repositories": all_repositories,
    })))
}

async fn remove_installation(
    State(state): State<AppState>,
    AuthUser(user): AuthUser,
    Path(installation_id): Path<i64>,
) -> HttpResult<impl IntoResponse> {
    if !GithubConnectionRepo::user_has_installation(
        &state.storage.db_pool,
        &user.id,
        installation_id,
    )
    .await?
    {
        return Err(HttpError::not_found("GitHub installation not found"));
    }
    let github_client = get_github_client(&state).await?;
    match github_client.delete_installation(installation_id).await {
        Ok(()) | Err(GithubError::AccessRevoked) => {}
        Err(error) => return Err(HttpError::internal(anyhow::Error::from(error))),
    }

    GithubConnectionRepo::disconnect_installation(&state.storage.db_pool, installation_id).await?;

    Ok(StatusCode::NO_CONTENT)
}
