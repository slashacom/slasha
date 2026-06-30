use axum::{
    Json, Router,
    extract::{Query, State},
    http::StatusCode,
    response::{IntoResponse, Redirect},
    routing::{delete, get, patch, post},
};
use chrono::Utc;
use serde::{Deserialize, Serialize};
use slasha_db::{
    github_app_config::GithubAppConfig, repos::github_app_config::GithubAppConfigRepo,
    user::UserRole,
};

use crate::{
    AppState,
    connections::github::{create_state, verify_state},
    error::{HttpError, HttpResult},
    extractors::auth::AuthUser,
};

pub fn router() -> Router<AppState> {
    Router::new()
        .route("/", get(get_setup_status))
        .route("/begin", post(begin_setup))
        .route("/", patch(update_credentials))
        .route("/", delete(delete_setup))
}

pub fn callback_router() -> Router<AppState> {
    Router::new().route("/callback", get(setup_callback))
}

#[derive(Serialize)]
struct SetupStatus {
    configured: bool,
    app_id: Option<String>,
    created_at: Option<chrono::NaiveDateTime>,
}

async fn get_setup_status(
    State(state): State<AppState>,
    AuthUser(user): AuthUser,
) -> HttpResult<impl IntoResponse> {
    if user.role != UserRole::Admin {
        return Err(HttpError::forbidden("Admin access required"));
    }
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
    if user.role != UserRole::Admin {
        return Err(HttpError::forbidden("Admin access required"));
    }

    let platform_domain = &state.config.platform_domain;
    let base_url = format!("https://{}", platform_domain);

    let state_token = create_state(&user.id, "/settings/connections", &state.config.jwt_secret)?;

    let manifest = serde_json::json!({
        "name": format!("Slasha ({})", platform_domain),
        "url": base_url,
        "hook_attributes": {
            "url": format!("{}/api/github-app/webhook", base_url),
            "active": true
        },
        "redirect_url": format!("{}/api/github-app/setup/callback", base_url),
        "callback_urls": [format!("{}/api/github-app/callback", base_url)],
        "public": false,
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
struct CallbackQuery {
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
    Query(query): Query<CallbackQuery>,
) -> HttpResult<Redirect> {
    let (user_id, redirect_to) = verify_state(&query.state, &state.config.jwt_secret)
        .map_err(|_| HttpError::bad_request("Invalid or expired setup state"))?;

    slasha_db::repos::user::UserRepo::find_by_id(&state.storage.db_pool, &user_id)
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

    Ok(Redirect::to(&format!("{}?setup=success", redirect_to)))
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
    if user.role != UserRole::Admin {
        return Err(HttpError::forbidden("Admin access required"));
    }

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
    if user.role != UserRole::Admin {
        return Err(HttpError::forbidden("Admin access required"));
    }

    GithubAppConfigRepo::delete(&state.storage.db_pool).await?;
    state.clear_github_client().await;

    tracing::info!(user_id = %user.id, "github app config deleted");

    Ok(StatusCode::NO_CONTENT)
}
