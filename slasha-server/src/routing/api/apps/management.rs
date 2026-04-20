use axum::{
    Json, Router,
    extract::{Path, State},
    response::IntoResponse,
    routing::{delete, get, post},
};
use chrono::Utc;
use diesel::prelude::*;
use serde::Deserialize;
use tokio::process::Command;
use uuid::Uuid;

use crate::{
    AppState,
    docker::run::delete_deployment_container,
    error::{Error, Result},
    extractors::auth::AuthUser,
    utils::slugify,
};

use super::utils::lookup_app_for_user;

use models::{
    app::{App, AppMember, AppMemberRole},
    deployment::Deployment,
    schema::{app_members, apps, deployments},
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
    State(state): State<AppState>,
    auth: AuthUser,
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

    let mut conn = state
        .db_pool
        .get()
        .map_err(|e| Error::Internal(anyhow::anyhow!("DB pool error: {}", e)))?;

    let existing = apps::table
        .filter(apps::slug.eq(&slug))
        .first::<App>(&mut conn)
        .optional()?;

    if existing.is_some() {
        return Err(Error::BadRequest(format!(
            "An app with the slug '{}' already exists",
            slug
        )));
    }

    let repo_path = state
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
        user_id: auth.0.id.clone(),
        role: AppMemberRole::Owner,
        added_at: now,
    };

    diesel::insert_into(apps::table)
        .values(&new_app)
        .execute(&mut conn)?;

    diesel::insert_into(app_members::table)
        .values(&new_member)
        .execute(&mut conn)?;

    Ok(Json(serde_json::json!({
        "app": new_app,
    })))
}

async fn get_app(
    State(state): State<AppState>,
    auth: AuthUser,
    Path(slug): Path<String>,
) -> Result<impl IntoResponse> {
    let app = lookup_app_for_user(&state, &slug, &auth.0.id)?;

    Ok(Json(serde_json::json!({
        "app": app,
    })))
}

async fn delete_app(
    State(state): State<AppState>,
    auth: AuthUser,
    Path(slug): Path<String>,
) -> Result<impl IntoResponse> {
    let mut conn = state
        .db_pool
        .get()
        .map_err(|e| Error::Internal(anyhow::anyhow!("DB pool error: {}", e)))?;

    let app = lookup_app_for_user(&state, &slug, &auth.0.id)?;

    let membership = app_members::table
        .filter(app_members::app_id.eq(&app.id))
        .filter(app_members::user_id.eq(&auth.0.id))
        .first::<AppMember>(&mut conn)
        .optional()?
        .ok_or_else(|| Error::NotFound(format!("App '{}' not found", slug)))?;

    if membership.role != AppMemberRole::Owner {
        return Err(Error::BadRequest("Only app owners can delete apps".into()));
    }

    let deployments: Vec<Deployment> = deployments::table
        .filter(deployments::app_id.eq(&app.id))
        .load(&mut conn)?;

    for dep in deployments {
        if let Err(e) = delete_deployment_container(
            &state.docker,
            &state.port_pool,
            &state.deployment_broadcaster,
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

    diesel::delete(apps::table.filter(apps::id.eq(&app.id))).execute(&mut conn)?;

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

async fn list_apps(State(state): State<AppState>, auth: AuthUser) -> Result<impl IntoResponse> {
    let mut conn = state
        .db_pool
        .get()
        .map_err(|e| Error::Internal(anyhow::anyhow!("DB pool error: {}", e)))?;

    let user_app_ids: Vec<String> = app_members::table
        .filter(app_members::user_id.eq(&auth.0.id))
        .select(app_members::app_id)
        .load(&mut conn)?;

    let user_apps: Vec<App> = apps::table
        .filter(apps::id.eq_any(&user_app_ids))
        .order(apps::created_at.desc())
        .load(&mut conn)?;

    Ok(Json(serde_json::json!({
        "apps": user_apps,
    })))
}
