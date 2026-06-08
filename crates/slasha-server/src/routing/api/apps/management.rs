use anyhow::Context;
use axum::{
    Json, Router,
    extract::{Path, State},
    response::IntoResponse,
    routing::{delete, get, post},
};
use bollard::Docker;
use chrono::Utc;
use serde::Deserialize;
use slasha_db::{
    DbPool,
    app::{App, AppMember, AppMemberRole},
    repos::{app::AppRepo, app_scale::AppScaleRepo, service::ServiceRepo},
};
use tokio::process::Command;
use uuid::Uuid;

use crate::{
    docker::{
        deployment::{delete_app_volumes, delete_deployment_processes},
        network::{create_app_network, delete_app_network},
        service::delete_service,
    },
    error::{HttpError, HttpResult},
    extractors::auth::AuthUser,
    state::{AppState, Runtime, Storage},
    utils::slugify,
};

pub fn router() -> Router<AppState> {
    Router::new()
        .route("/", post(create_app))
        .route("/", get(list_apps))
        .route("/{slug}", get(get_app))
        .route("/{slug}/scales", get(list_scales))
        .route("/{slug}", delete(delete_app))
}

#[derive(Deserialize)]
struct CreateAppReq {
    name: String,
}

async fn create_app(
    State(docker): State<Docker>,
    State(storage): State<Storage>,
    AuthUser(user): AuthUser,
    Json(payload): Json<CreateAppReq>,
) -> HttpResult<impl IntoResponse> {
    let name = payload.name.trim().to_string();
    if name.is_empty() {
        return Err(HttpError::bad_request("App name cannot be empty"));
    }

    let slug = slugify(&name);
    if slug.is_empty() {
        return Err(HttpError::bad_request(
            "App name must contain alphanumeric characters",
        ));
    }

    if AppRepo::slug_exists(&storage.db_pool, &slug).await? {
        return Err(HttpError::bad_request(format!(
            "An app with the slug '{}' already exists",
            slug
        )));
    }

    let repo_path = storage
        .repos_dir
        .join(format!("{}.git", slug))
        .to_str()
        .ok_or_else(|| HttpError::internal(anyhow::anyhow!("Invalid repo path")))?
        .to_string();

    let git_status = Command::new("git")
        .arg("init")
        .arg("--bare")
        .arg("--initial-branch=main")
        .arg(&repo_path)
        .output()
        .await
        .context("Failed to init bare repo")?;

    if !git_status.status.success() {
        let stderr = String::from_utf8_lossy(&git_status.stderr);
        return Err(anyhow::anyhow!("git init --bare failed: {}", stderr).into());
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

    create_app_network(&docker, &app_id).await?;

    Ok(Json(serde_json::json!({
        "app": new_app,
    })))
}

async fn get_app(
    State(storage): State<Storage>,
    AuthUser(user): AuthUser,
    Path(slug): Path<String>,
) -> HttpResult<impl IntoResponse> {
    let app = AppRepo::find_by_slug_for_user(&storage.db_pool, &slug, &user.id).await?;

    Ok(Json(serde_json::json!({
        "app": app,
    })))
}

async fn delete_app(
    State(docker): State<Docker>,
    State(db_pool): State<DbPool>,
    State(runtime): State<Runtime>,
    AuthUser(user): AuthUser,
    Path(slug): Path<String>,
) -> HttpResult<impl IntoResponse> {
    let app = AppRepo::find_by_slug_for_user(&db_pool, &slug, &user.id).await?;

    if !AppRepo::is_owner(&db_pool, &app.id, &user.id).await? {
        return Err(HttpError::bad_request("Only app owners can delete apps"));
    }

    let app_services = ServiceRepo::list_for_app(&db_pool, &app.id).await?;

    for svc in app_services {
        if let Err(e) = delete_service(&docker, &db_pool, &runtime.log_manager, &app, &svc).await {
            tracing::warn!(
                service_id = %svc.id,
                error = %e,
                "Failed to delete service"
            );
        }
    }

    let deployments = AppRepo::delete(&db_pool, &app.id).await?;

    for dep in deployments {
        if let Err(e) = delete_deployment_processes(
            &docker,
            &runtime.proxy_sync_trigger,
            &runtime.log_manager,
            &app,
            &dep,
        )
        .await
        {
            tracing::warn!(
                deployment_id = %dep.id,
                error = %e,
                "Failed to delete container for deployment"
            );
        }
    }

    delete_app_network(&docker, &app.id).await?;

    if let Err(e) = delete_app_volumes(&docker, &app.id).await {
        tracing::warn!(
            app_id = %app.id,
            error = ?e,
            "Failed to clean up volumes for app"
        );
    }

    if let Err(e) = runtime.log_manager.delete_app_logs(&app.slug).await {
        tracing::warn!(
            app_slug = %app.slug,
            error = ?e,
            "Failed to delete logs for app"
        );
    }

    let repo_path = std::path::Path::new(&app.repo_path);
    if repo_path.exists() {
        tokio::fs::remove_dir_all(repo_path)
            .await
            .context("Failed to remove repo")?;
    }

    Ok(Json(serde_json::json!({
        "deleted": true,
        "slug": slug,
    })))
}

async fn list_apps(
    State(storage): State<Storage>,
    AuthUser(user): AuthUser,
) -> HttpResult<impl IntoResponse> {
    let user_apps = AppRepo::list_for_user(&storage.db_pool, &user.id).await?;

    Ok(Json(serde_json::json!({
        "apps": user_apps,
    })))
}

async fn list_scales(
    State(storage): State<Storage>,
    AuthUser(user): AuthUser,
    Path(slug): Path<String>,
) -> HttpResult<impl IntoResponse> {
    let app = AppRepo::find_by_slug_for_user(&storage.db_pool, &slug, &user.id).await?;
    let scales = AppScaleRepo::list_for_app(&storage.db_pool, &app.id).await?;

    Ok(Json(serde_json::json!({ "scales": scales })))
}
