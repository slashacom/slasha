use std::collections::HashMap;

use anyhow::Context;
use axum::{
    Json, Router,
    extract::State,
    response::IntoResponse,
    routing::{delete, get, post, put},
};
use bollard::Docker;
use chrono::Utc;
use serde::Deserialize;
use slasha_db::{
    DbPool,
    app::{App, AppMember, AppMemberRole},
    deployment::{Deployment, DeploymentStatus},
    repos::{
        app::AppRepo, app_domain::AppDomainRepo, app_scale::AppScaleRepo,
        deployment::DeploymentRepo, service::ServiceRepo,
    },
    user::UserRole,
};
use tokio::process::Command;
use uuid::Uuid;

use crate::{
    docker::{
        deployment::{remove_app_volumes, remove_deployment_processes},
        network::{create_app_network, remove_app_network},
        service::remove_service_container,
    },
    error::{HttpError, HttpResult},
    extractors::{app::ActiveApp, auth::AuthUser},
    state::{AppState, Config, Runtime, Storage},
    utils::slugify,
};

pub fn router() -> Router<AppState> {
    Router::new()
        .route("/", post(create_app))
        .route("/", get(list_apps))
        .route("/{slug}", get(get_app))
        .route("/{slug}/scales", get(list_scales))
        .route("/{slug}", delete(delete_app))
        .route("/{slug}/settings", put(update_settings))
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
        auto_deploy: true,
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
    State(config): State<Config>,
    ActiveApp { app, .. }: ActiveApp,
) -> HttpResult<impl IntoResponse> {
    let domains = AppDomainRepo::list_for_app(&storage.db_pool, &app.id).await?;
    let url = match domains.first() {
        Some(domain) => format!(
            "https://{}",
            domain
                .domain
                .trim_start_matches("http://")
                .trim_start_matches("https://")
        ),
        None => {
            let scheme = if config.platform_domain.contains("localhost") {
                "http"
            } else {
                "https"
            };
            format!("{}://{}.{}", scheme, app.slug, config.platform_domain)
        }
    };

    Ok(Json(serde_json::json!({
        "app": app,
        "url": url,
    })))
}

async fn delete_app(
    State(docker): State<Docker>,
    State(db_pool): State<DbPool>,
    State(runtime): State<Runtime>,
    ActiveApp { app, user }: ActiveApp,
) -> HttpResult<impl IntoResponse> {
    if user.role != UserRole::Admin && !AppRepo::is_owner(&db_pool, &app.id, &user.id).await? {
        return Err(HttpError::bad_request("Only app owners can delete apps"));
    }

    let app_services = ServiceRepo::list_for_app(&db_pool, &app.id).await?;
    let deployments = AppRepo::delete(&db_pool, &app.id).await?;

    let app_clone = app.clone();
    let app_slug = app.slug.clone();
    let app_id = app.id.clone();

    tokio::spawn(async move {
        for service in app_services {
            if let Err(e) =
                remove_service_container(&docker, &runtime.log_manager, &app_clone, &service, true)
                    .await
            {
                tracing::warn!(
                    service_id = %service.id,
                    error = %e,
                    "Failed to delete service"
                );
            }
        }

        for dep in deployments {
            if let Err(e) = remove_deployment_processes(
                &docker,
                &runtime.proxy_sync_trigger,
                &runtime.log_manager,
                &app_clone,
                &dep,
            )
            .await
            {
                tracing::warn!(
                    deployment_id = %dep.id,
                    error = %e,
                    "Failed to remove deployment processes"
                );
            }
        }

        if let Err(e) = remove_app_network(&docker, &app_id).await {
            tracing::warn!(
                app_id = %app_id,
                error = ?e,
                "Failed to remove app network"
            );
        }

        if let Err(e) = remove_app_volumes(&docker, &app_id).await {
            tracing::warn!(
                app_id = %app_id,
                error = ?e,
                "Failed to clean up volumes for app"
            );
        }

        if let Err(e) = runtime.log_manager.delete_app_logs(&app_slug).await {
            tracing::warn!(
                app_slug = %app_slug,
                error = ?e,
                "Failed to delete logs for app"
            );
        }

        let repo_path = std::path::Path::new(&app_clone.repo_path);
        if repo_path.exists()
            && let Err(e) = tokio::fs::remove_dir_all(repo_path).await
        {
            tracing::warn!(
                app_slug = %app_slug,
                error = ?e,
                "Failed to remove repo"
            );
        }
    });

    Ok(Json(serde_json::json!({
        "deleted": true,
        "slug": app.slug,
    })))
}

fn derive_runtime_status(deployments: &[Deployment]) -> &'static str {
    if deployments
        .iter()
        .any(|d| d.status == DeploymentStatus::Running)
    {
        return "running";
    }
    match deployments.first().map(|d| d.status) {
        Some(DeploymentStatus::Building) | Some(DeploymentStatus::Pending) => "deploying",
        Some(DeploymentStatus::Failed) => "failed",
        _ => "idle",
    }
}

async fn list_apps(
    State(storage): State<Storage>,
    State(config): State<Config>,
    AuthUser(user): AuthUser,
) -> HttpResult<impl IntoResponse> {
    let user_apps = AppRepo::list_for_user(&storage.db_pool, &user.id).await?;

    let scheme = if config.platform_domain.contains("localhost") {
        "http"
    } else {
        "https"
    };

    let app_ids = user_apps.iter().map(|app| app.id.clone()).collect();
    let domains = AppDomainRepo::list_for_apps(&storage.db_pool, app_ids).await?;
    let mut primary_domains: HashMap<String, String> = HashMap::new();
    for domain in domains {
        primary_domains
            .entry(domain.app_id)
            .or_insert(domain.domain);
    }

    let mut items = Vec::with_capacity(user_apps.len());
    for app in user_apps {
        let deployments = DeploymentRepo::list_for_app(&storage.db_pool, &app.id).await?;
        let url = match primary_domains.get(&app.id) {
            Some(domain) => format!(
                "https://{}",
                domain
                    .trim_start_matches("http://")
                    .trim_start_matches("https://")
            ),
            None => format!("{}://{}.{}", scheme, app.slug, config.platform_domain),
        };
        items.push(serde_json::json!({
            "app": app,
            "url": url,
            "runtime_status": derive_runtime_status(&deployments),
        }));
    }

    Ok(Json(serde_json::json!({
        "apps": items,
    })))
}

async fn list_scales(
    State(storage): State<Storage>,
    ActiveApp { app, .. }: ActiveApp,
) -> HttpResult<impl IntoResponse> {
    let scales = AppScaleRepo::list_for_app(&storage.db_pool, &app.id).await?;

    Ok(Json(serde_json::json!({ "scales": scales })))
}

#[derive(Deserialize)]
struct UpdateSettingsReq {
    auto_deploy: bool,
}

async fn update_settings(
    State(storage): State<Storage>,
    ActiveApp { app, .. }: ActiveApp,
    Json(payload): Json<UpdateSettingsReq>,
) -> HttpResult<impl IntoResponse> {
    AppRepo::update_auto_deploy(&storage.db_pool, &app.id, payload.auto_deploy).await?;

    Ok(Json(serde_json::json!({
        "success": true,
    })))
}
