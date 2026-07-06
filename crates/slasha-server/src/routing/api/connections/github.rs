use axum::{
    Json, Router,
    extract::{Path, Query, State},
    http::StatusCode,
    response::{IntoResponse, Redirect},
    routing::{delete, get, patch, post},
};
use chrono::Utc;
use serde::{Deserialize, Serialize};
use slasha_db::{
    github_app_config::GithubAppConfig,
    github_connection::GithubInstallation,
    repos::{
        github_app_config::GithubAppConfigRepo, github_connection::GithubConnectionRepo,
        user::UserRepo,
    },
};

use crate::{
    AppState, HttpError, HttpResult,
    connections::{GithubError, create_state, handle_webhook, verify_state},
    extractors::auth::AuthUser,
    middleware::admin::admin_middleware,
};

pub fn router(state: AppState) -> Router<AppState> {
    let app_routes = Router::new()
        .route("/status", get(get_app_status))
        .route("/install", post(install_app))
        .route("/callback", get(app_callback))
        .route("/repositories", get(get_app_repositories))
        .route(
            "/installations/{installation_id}",
            delete(remove_installation),
        )
        .route(
            "/installations/{installation_id}/repositories/{repository_id}/branches",
            get(get_github_app_branches),
        )
        .route("/webhook", post(handle_webhook));

    let setup_routes = Router::new()
        .route("/", get(get_setup_status))
        .route("/begin", post(begin_setup))
        .route("/", patch(update_credentials))
        .route("/", delete(delete_setup))
        .route_layer(axum::middleware::from_fn_with_state(
            state.clone(),
            admin_middleware,
        ))
        .route("/callback", get(setup_callback));

    Router::new()
        .nest("/github", app_routes)
        .nest("/github/setup", setup_routes)
}

#[derive(Serialize)]
struct SetupStatus {
    configured: bool,
    app_id: Option<String>,
    created_at: Option<chrono::NaiveDateTime>,
}

async fn get_setup_status(
    State(state): State<AppState>,
    AuthUser(_user): AuthUser,
) -> HttpResult<impl IntoResponse> {
    let config = GithubAppConfigRepo::get(&state.storage.db_pool).await?;
    Ok(Json(match config {
        Some(c) => SetupStatus {
            configured: true,
            app_id: Some(c.app_id),
            created_at: Some(c.created_at),
        },
        None => SetupStatus {
            configured: false,
            app_id: None,
            created_at: None,
        },
    }))
}

async fn begin_setup(
    State(state): State<AppState>,
    AuthUser(user): AuthUser,
) -> HttpResult<impl IntoResponse> {
    let platform_domain = &state.config.platform_domain;
    let base_url = format!("https://{}", platform_domain);

    let state_token = create_state(
        &user.id,
        "/settings/connections?setup=success",
        &state.config.jwt_secret,
    )?;

    let manifest = serde_json::json!({
        "name": format!("Slasha ({})", platform_domain),
        "url": base_url,
        "hook_attributes": {
            "url": format!("{}/api/connections/github/webhook", base_url),
            "active": true
        },
        "redirect_url": format!("{}/api/connections/github/setup/callback", base_url),
        "callback_urls": [format!("{}/api/connections/github/callback", base_url)],
        "public": true,
        "request_oauth_on_install": true,
        "default_permissions": {
            "contents": "read",
            "metadata": "read"
        },
        "default_events": ["push"]
    });

    Ok(Json(serde_json::json!({
        "github_url": format!("https://github.com/settings/apps/new?state={}", urlencoding::encode(&state_token)),
        "manifest": manifest.to_string()
    })))
}

#[derive(Deserialize)]
struct SetupCallbackQuery {
    code: String,
    state: String,
}

#[derive(Deserialize)]
struct ManifestConversion {
    id: i64,
    client_id: String,
    client_secret: String,
    pem: String,
    webhook_secret: String,
}

async fn setup_callback(
    State(state): State<AppState>,
    Query(query): Query<SetupCallbackQuery>,
) -> HttpResult<Redirect> {
    let (user_id, redirect_to) = verify_state(&query.state, &state.config.jwt_secret)
        .map_err(|_| HttpError::bad_request("Invalid or expired setup state"))?;

    UserRepo::find_by_id(&state.storage.db_pool, &user_id)
        .await
        .map_err(|_| HttpError::unauthorized())?;

    let http = reqwest::Client::builder()
        .user_agent("slasha")
        .build()
        .map_err(|e| HttpError::internal(anyhow::anyhow!(e)))?;

    let conversion = http
        .post(format!(
            "https://api.github.com/app-manifests/{}/conversions",
            query.code
        ))
        .header("Accept", "application/vnd.github+json")
        .send()
        .await
        .map_err(|e| HttpError::internal(anyhow::anyhow!(e)))?
        .error_for_status()
        .map_err(|e| HttpError::internal(anyhow::anyhow!(e)))?
        .json::<ManifestConversion>()
        .await
        .map_err(|e| HttpError::internal(anyhow::anyhow!(e)))?;

    let now = Utc::now().naive_utc();
    let config = GithubAppConfig {
        id: "default".to_string(),
        app_id: conversion.id.to_string(),
        client_id: conversion.client_id,
        client_secret: conversion.client_secret,
        private_key: conversion.pem,
        webhook_secret: conversion.webhook_secret,
        created_at: now,
        updated_at: now,
    };

    GithubAppConfigRepo::upsert(&state.storage.db_pool, config).await?;

    state
        .reload_github_client()
        .await
        .map_err(|e| HttpError::internal(anyhow::anyhow!(e)))?;

    tracing::info!(user_id = %user_id, "github app configured via manifest flow");

    Ok(Redirect::to(&redirect_to))
}

#[derive(Deserialize)]
struct UpdateCredentialsReq {
    app_id: String,
    client_id: String,
    client_secret: String,
    private_key: String,
    webhook_secret: String,
}

async fn update_credentials(
    State(state): State<AppState>,
    AuthUser(user): AuthUser,
    Json(payload): Json<UpdateCredentialsReq>,
) -> HttpResult<impl IntoResponse> {
    jsonwebtoken::EncodingKey::from_rsa_pem(payload.private_key.replace("\\n", "\n").as_bytes())
        .map_err(|_| HttpError::bad_request("Invalid RSA private key PEM"))?;

    let now = Utc::now().naive_utc();
    let config = GithubAppConfig {
        id: "default".to_string(),
        app_id: payload.app_id,
        client_id: payload.client_id,
        client_secret: payload.client_secret,
        private_key: payload.private_key,
        webhook_secret: payload.webhook_secret,
        created_at: now,
        updated_at: now,
    };

    let saved = GithubAppConfigRepo::upsert(&state.storage.db_pool, config).await?;

    state
        .reload_github_client()
        .await
        .map_err(|e| HttpError::internal(anyhow::anyhow!(e)))?;

    tracing::info!(user_id = %user.id, "github app credentials updated manually");

    Ok(Json(serde_json::json!({
        "configured": true,
        "app_id": saved.app_id,
        "created_at": saved.created_at,
    })))
}

async fn delete_setup(
    State(state): State<AppState>,
    AuthUser(user): AuthUser,
) -> HttpResult<impl IntoResponse> {
    GithubAppConfigRepo::delete(&state.storage.db_pool).await?;
    state.clear_github_client().await;

    tracing::info!(user_id = %user.id, "github app config deleted");

    Ok(StatusCode::NO_CONTENT)
}

async fn get_github_client(state: &AppState) -> HttpResult<crate::connections::GithubClient> {
    state
        .github_client()
        .await
        .ok_or_else(|| HttpError::not_found("GitHub integration is disabled"))
}

async fn get_app_status(State(state): State<AppState>) -> Json<serde_json::Value> {
    Json(serde_json::json!({
        "enabled": state.github_client().await.is_some(),
    }))
}

#[derive(Deserialize)]
struct InstallReq {
    redirect_to: String,
}

async fn install_app(
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

async fn app_callback(
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

async fn get_app_repositories(
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

async fn get_github_app_branches(
    State(state): State<AppState>,
    AuthUser(user): AuthUser,
    Path((installation_id, repository_id)): Path<(i64, i64)>,
) -> HttpResult<impl IntoResponse> {
    let github = state
        .github_client()
        .await
        .ok_or_else(|| HttpError::not_found("GitHub integration is disabled"))?;

    if !GithubConnectionRepo::user_has_installation(
        &state.storage.db_pool,
        &user.id,
        installation_id,
    )
    .await?
    {
        return Err(HttpError::forbidden(
            "GitHub installation is not connected to this user",
        ));
    }

    let repo = github
        .get_repository(installation_id, repository_id)
        .await
        .map_err(|error| HttpError::bad_request(format!("Failed to fetch repo: {}", error)))?;

    let branches = github
        .get_branches(installation_id, repository_id)
        .await
        .map_err(|error| HttpError::bad_request(format!("Failed to fetch branches: {}", error)))?;

    Ok(Json(serde_json::json!({
        "default_branch": repo.default_branch,
        "branches": branches,
    })))
}
