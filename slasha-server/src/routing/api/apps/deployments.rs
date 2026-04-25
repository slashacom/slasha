use axum::{
    Json, Router,
    extract::{Path, State},
    response::{
        IntoResponse,
        sse::{Event, KeepAlive, Sse},
    },
    routing::{delete, get, post},
};
use chrono::Utc;
use diesel::prelude::*;
use futures_util::{StreamExt, stream};
use serde::Deserialize;
use tokio::sync::broadcast;
use uuid::Uuid;

use crate::{
    docker::pipeline::run_deployment,
    docker::run::{delete_deployment_container, stop_deployment_container},
    error::{Error, Result},
    extractors::auth::AuthUser,
    state::{AppState, Clients, Runtime, Storage},
};
use models::{
    app::App,
    deployment::{Deployment, DeploymentStatus},
    schema::deployments,
};

use super::utils::lookup_app_for_user;

pub fn router() -> Router<AppState> {
    Router::new()
        .route("/", post(trigger_deploy).get(list_deployments))
        .route("/{deployment_id}", get(get_deployment))
        .route("/{deployment_id}/logs", get(stream_logs))
        .route("/{deployment_id}/stop", post(stop_deployment))
        .route("/{deployment_id}/restart", post(restart_deployment))
        .route("/{deployment_id}", delete(delete_deployment))
}

#[derive(Deserialize)]
struct TriggerDeployReq {
    commit_sha: Option<String>,
}

async fn trigger_deploy(
    State(clients): State<Clients>,
    State(storage): State<Storage>,
    State(runtime): State<Runtime>,
    AuthUser(user): AuthUser,
    Path(slug): Path<String>,
    Json(payload): Json<TriggerDeployReq>,
) -> Result<impl IntoResponse> {
    let mut conn = storage.db_pool.get()?;

    let is_running: bool = diesel::select(diesel::dsl::exists(
        deployments::table.filter(deployments::status.eq(DeploymentStatus::Running)),
    ))
    .get_result(&mut conn)?;

    if is_running {
        return Err(Error::BadRequest(
            "A deployment is already running".to_string(),
        ));
    }

    let app = lookup_app_for_user(&storage, &slug, &user.id)?;

    let (commit_sha, commit_message) = match payload.commit_sha {
        Some(sha) => {
            let msg = resolve_commit_message(&app.repo_path, &sha)
                .await
                .map_err(|e| Error::BadRequest(format!("Invalid commit SHA: {}", e)))?;
            (sha, msg)
        }
        None => resolve_head_commit(&app, &app.default_branch)
            .await
            .map_err(|e| {
                Error::BadRequest(format!(
                    "Failed to resolve HEAD of '{}': {}",
                    app.default_branch, e
                ))
            })?,
    };

    let now = Utc::now().naive_utc();
    let deployment = Deployment {
        id: Uuid::new_v4().to_string(),
        app_id: app.id.clone(),
        commit_sha,
        commit_message,
        status: DeploymentStatus::Pending,
        created_at: now,
        updated_at: now,
    };

    diesel::insert_into(deployments::table)
        .values(&deployment)
        .execute(&mut conn)?;

    tokio::spawn(run_deployment(
        clients.docker.clone(),
        storage.clone(),
        runtime.clone(),
        app,
        deployment.clone(),
    ));

    Ok(Json(serde_json::json!({ "deployment": deployment })))
}

async fn list_deployments(
    State(storage): State<Storage>,
    AuthUser(user): AuthUser,
    Path(slug): Path<String>,
) -> Result<impl IntoResponse> {
    let app = lookup_app_for_user(&storage, &slug, &user.id)?;

    let mut conn = storage.db_pool.get()?;

    let deps: Vec<Deployment> = deployments::table
        .filter(deployments::app_id.eq(&app.id))
        .order(deployments::created_at.desc())
        .load(&mut conn)?;

    Ok(Json(serde_json::json!({ "deployments": deps })))
}

async fn get_deployment(
    State(storage): State<Storage>,
    AuthUser(user): AuthUser,
    Path((slug, deployment_id)): Path<(String, String)>,
) -> Result<impl IntoResponse> {
    let app = lookup_app_for_user(&storage, &slug, &user.id)?;

    let mut conn = storage.db_pool.get()?;

    let deployment = deployments::table
        .filter(deployments::id.eq(&deployment_id))
        .filter(deployments::app_id.eq(&app.id))
        .first::<Deployment>(&mut conn)
        .optional()?
        .ok_or_else(|| Error::NotFound(format!("Deployment '{}' not found", deployment_id)))?;

    Ok(Json(serde_json::json!({ "deployment": deployment })))
}

async fn stop_deployment(
    State(clients): State<Clients>,
    State(storage): State<Storage>,
    State(runtime): State<Runtime>,
    AuthUser(user): AuthUser,
    Path((slug, deployment_id)): Path<(String, String)>,
) -> Result<impl IntoResponse> {
    let app = lookup_app_for_user(&storage, &slug, &user.id)?;

    let mut conn = storage.db_pool.get()?;

    let deployment = deployments::table
        .filter(deployments::id.eq(&deployment_id))
        .filter(deployments::app_id.eq(&app.id))
        .first::<Deployment>(&mut conn)
        .optional()?
        .ok_or_else(|| Error::NotFound(format!("Deployment '{}' not found", deployment_id)))?;

    if !matches!(
        deployment.status,
        DeploymentStatus::Running | DeploymentStatus::Building
    ) {
        return Err(Error::BadRequest(format!(
            "Deployment is already in state '{}'",
            deployment.status
        )));
    }

    drop(conn);

    stop_deployment_container(&clients.docker, &storage, &runtime, &app, &deployment)
        .await
        .map_err(|e| Error::Internal(anyhow::anyhow!("Failed to stop deployment: {}", e)))?;

    Ok(Json(serde_json::json!({
        "stopped": true,
        "deployment_id": deployment_id
    })))
}

async fn restart_deployment(
    State(clients): State<Clients>,
    State(storage): State<Storage>,
    State(runtime): State<Runtime>,
    AuthUser(user): AuthUser,
    Path((slug, deployment_id)): Path<(String, String)>,
) -> Result<impl IntoResponse> {
    let app = lookup_app_for_user(&storage, &slug, &user.id)?;

    let mut conn = storage.db_pool.get()?;

    let deployment = deployments::table
        .filter(deployments::id.eq(&deployment_id))
        .filter(deployments::app_id.eq(&app.id))
        .first::<Deployment>(&mut conn)
        .optional()?
        .ok_or_else(|| Error::NotFound(format!("Deployment '{}' not found", deployment_id)))?;

    delete_deployment_container(&clients.docker, &runtime, &app, &deployment)
        .await
        .map_err(|e| Error::Internal(anyhow::anyhow!("Failed to clean up container: {}", e)))?;

    let now = Utc::now().naive_utc();
    diesel::update(deployments::table.filter(deployments::id.eq(&deployment.id)))
        .set((
            deployments::status.eq(DeploymentStatus::Pending.to_string()),
            deployments::updated_at.eq(now),
        ))
        .execute(&mut conn)?;

    let mut updated_deployment = deployment.clone();
    updated_deployment.status = DeploymentStatus::Pending;
    updated_deployment.updated_at = now;

    tokio::spawn(run_deployment(
        clients.docker.clone(),
        storage.clone(),
        runtime.clone(),
        app,
        updated_deployment.clone(),
    ));

    Ok(Json(
        serde_json::json!({ "deployment": updated_deployment }),
    ))
}

async fn stream_logs(
    State(storage): State<Storage>,
    State(runtime): State<Runtime>,
    AuthUser(user): AuthUser,
    Path((slug, deployment_id)): Path<(String, String)>,
) -> Result<
    Sse<impl futures_util::Stream<Item = std::result::Result<Event, std::convert::Infallible>>>,
> {
    let app = lookup_app_for_user(&storage, &slug, &user.id)?;

    let mut conn = storage.db_pool.get()?;

    deployments::table
        .filter(deployments::id.eq(&deployment_id))
        .filter(deployments::app_id.eq(&app.id))
        .first::<Deployment>(&mut conn)
        .optional()?
        .ok_or_else(|| Error::NotFound(format!("Deployment '{}' not found", deployment_id)))?;

    let historical = runtime
        .deployment_broadcaster
        .get_historical(&deployment_id)
        .await;

    let historical_stream = stream::iter(
        historical
            .into_iter()
            .map(|log| Ok(Event::default().data(log))),
    );

    let live_rx = runtime.deployment_broadcaster.subscribe(&deployment_id);

    let live_stream = futures_util::stream::unfold(live_rx, move |mut rx| async move {
        match rx.recv().await {
            Ok(msg) => Some((Ok(Event::default().data(msg)), rx)),
            Err(broadcast::error::RecvError::Lagged(_)) => {
                Some((Ok(Event::default().data("lagged")), rx))
            }
            Err(broadcast::error::RecvError::Closed) => None,
        }
    });

    let combined = historical_stream.chain(live_stream);

    Ok(Sse::new(combined).keep_alive(KeepAlive::default()))
}

async fn delete_deployment(
    State(clients): State<Clients>,
    State(storage): State<Storage>,
    State(runtime): State<Runtime>,
    AuthUser(user): AuthUser,
    Path((slug, deployment_id)): Path<(String, String)>,
) -> Result<impl IntoResponse> {
    let app = lookup_app_for_user(&storage, &slug, &user.id)?;

    let mut conn = storage.db_pool.get()?;

    let deployment = deployments::table
        .filter(deployments::id.eq(&deployment_id))
        .filter(deployments::app_id.eq(&app.id))
        .first::<Deployment>(&mut conn)
        .optional()?
        .ok_or_else(|| Error::NotFound(format!("Deployment '{}' not found", deployment_id)))?;

    delete_deployment_container(&clients.docker, &runtime, &app, &deployment)
        .await
        .map_err(|e| Error::Internal(anyhow::anyhow!("Failed to clean up container: {}", e)))?;

    diesel::delete(deployments::table.filter(deployments::id.eq(&deployment.id)))
        .execute(&mut conn)?;

    Ok(Json(serde_json::json!({
        "deleted": true,
        "deployment_id": deployment_id
    })))
}

async fn resolve_commit_message(repo_path: &str, sha: &str) -> anyhow::Result<String> {
    let out = tokio::process::Command::new("git")
        .args(["log", "-1", "--format=%s", sha])
        .current_dir(repo_path)
        .output()
        .await?;

    if !out.status.success() {
        anyhow::bail!("{}", String::from_utf8_lossy(&out.stderr));
    }

    Ok(String::from_utf8(out.stdout)?.trim().to_string())
}

async fn resolve_head_commit(app: &App, branch: &str) -> anyhow::Result<(String, String)> {
    let out = tokio::process::Command::new("git")
        .args(["log", "-1", "--format=%H|%s", branch])
        .current_dir(&app.repo_path)
        .output()
        .await?;

    if !out.status.success() {
        anyhow::bail!("{}", String::from_utf8_lossy(&out.stderr));
    }

    let raw = String::from_utf8(out.stdout)?.trim().to_string();
    let (sha, msg) = raw
        .split_once('|')
        .ok_or_else(|| anyhow::anyhow!("Unexpected git log output: {}", raw))?;

    Ok((sha.to_string(), msg.to_string()))
}
