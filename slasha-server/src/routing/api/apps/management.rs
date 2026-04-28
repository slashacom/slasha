use axum::{
    Json, Router,
    extract::{Path, State},
    response::IntoResponse,
    routing::{delete, get, post},
};
use chrono::Utc;
use serde::Deserialize;
use slasha_db::{
    app::{App, AppMember, AppMemberRole},
    repos::{app::AppRepo, service::ServiceRepo},
};
use tokio::process::Command;
use uuid::Uuid;

use crate::{
    docker::{
        network::{create_app_network, delete_app_network},
        run::delete_deployment_container,
        services::delete_service,
    },
    error::{Error, Result},
    extractors::auth::AuthUser,
    state::{AppState, Clients, Runtime, Storage},
    utils::slugify,
};

pub fn router() -> Router<AppState> {
    Router::new()
        .route("/", post(create_app))
        .route("/", get(list_apps))
        .route("/{slug}", get(get_app))
        .route("/{slug}", delete(delete_app))
}

#[derive(Deserialize)]
struct CreateAppReq {
    name: String,
}

async fn create_app(
    State(clients): State<Clients>,
    State(storage): State<Storage>,
    AuthUser(user): AuthUser,
    Json(payload): Json<CreateAppReq>,
) -> Result<impl IntoResponse> {
    let name = payload.name.trim().to_string();
    if name.is_empty() {
        return Err(Error::BadRequest("App name cannot be empty".into()));
    }

    let slug = slugify(&name);
    if slug.is_empty() {
        return Err(Error::BadRequest(
            "App name must contain alphanumeric characters".into(),
        ));
    }

    if AppRepo::slug_exists(&storage.db_pool, &slug).await? {
        return Err(Error::BadRequest(format!(
            "An app with the slug '{}' already exists",
            slug
        )));
    }

    let repo_path = storage
        .repos_dir
        .join(format!("{}.git", slug))
        .to_str()
        .ok_or_else(|| Error::Internal(anyhow::anyhow!("Invalid repo path")))?
        .to_string();

    let git_status = Command::new("git")
        .arg("init")
        .arg("--bare")
        .arg("--initial-branch=main")
        .arg(&repo_path)
        .output()
        .await
        .map_err(|e| Error::Internal(anyhow::anyhow!("Failed to init bare repo: {}", e)))?;

    if !git_status.status.success() {
        let stderr = String::from_utf8_lossy(&git_status.stderr);
        return Err(Error::Internal(anyhow::anyhow!(
            "git init --bare failed: {}",
            stderr
        )));
    }

    let now = Utc::now().naive_utc();
    let app_id = Uuid::new_v4().to_string();

    let new_app = App {
        id: app_id.clone(),
        slug: slug.clone(),
        name: name.clone(),
        repo_path,
        default_branch: "main".into(),
        status: "idle".into(),
        created_at: now,
    };

    let new_member = AppMember {
        app_id: app_id.clone(),
        user_id: user.id.clone(),
        role: AppMemberRole::Owner,
        added_at: now,
    };

    let new_app = AppRepo::create(&storage.db_pool, new_app, new_member).await?;

    create_app_network(&clients.docker, &app_id).await?;

    Ok(Json(serde_json::json!({
        "app": new_app,
    })))
}

async fn get_app(
    State(storage): State<Storage>,
    AuthUser(user): AuthUser,
    Path(slug): Path<String>,
) -> Result<impl IntoResponse> {
    let app = AppRepo::find_by_slug_for_user(&storage.db_pool, &slug, &user.id).await?;

    Ok(Json(serde_json::json!({
        "app": app,
    })))
}

async fn delete_app(
    State(clients): State<Clients>,
    State(storage): State<Storage>,
    State(runtime): State<Runtime>,
    AuthUser(user): AuthUser,
    Path(slug): Path<String>,
) -> Result<impl IntoResponse> {
    let app = AppRepo::find_by_slug_for_user(&storage.db_pool, &slug, &user.id).await?;

    if !AppRepo::is_owner(&storage.db_pool, &app.id, &user.id).await? {
        return Err(Error::BadRequest("Only app owners can delete apps".into()));
    }

    let app_services = ServiceRepo::list_for_app(&storage.db_pool, &app.id).await?;

    for svc in app_services {
        if let Err(e) = delete_service(
            &clients.docker,
            &storage.db_pool,
            &runtime.log_manager,
            &app,
            &svc,
        )
        .await
        {
            tracing::warn!("Failed to delete service {}: {}", svc.id, e);
        }
    }

    let deployments = AppRepo::delete(&storage.db_pool, &app.id).await?;

    for dep in deployments {
        if let Err(e) = delete_deployment_container(
            &clients.docker,
            &runtime.port_pool,
            &runtime.proxy_reconcile,
            &runtime.log_manager,
            &app,
            &dep,
        )
        .await
        {
            tracing::warn!(
                "Failed to delete container for deployment {}: {}",
                dep.id,
                e
            );
        }
    }

    delete_app_network(&clients.docker, &app.id).await?;

    let repo_path = std::path::Path::new(&app.repo_path);
    if repo_path.exists() {
        tokio::fs::remove_dir_all(repo_path)
            .await
            .map_err(|e| Error::Internal(anyhow::anyhow!("Failed to remove repo: {}", e)))?;
    }

    Ok(Json(serde_json::json!({
        "deleted": true,
        "slug": slug,
    })))
}

async fn list_apps(
    State(storage): State<Storage>,
    AuthUser(user): AuthUser,
) -> Result<impl IntoResponse> {
    let user_apps = AppRepo::list_for_user(&storage.db_pool, &user.id).await?;

    Ok(Json(serde_json::json!({
        "apps": user_apps,
    })))
}
