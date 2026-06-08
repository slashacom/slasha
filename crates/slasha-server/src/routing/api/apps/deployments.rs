use std::sync::Arc;

use axum::{
    Json, Router,
    extract::{Path, State},
    response::{
        IntoResponse,
        sse::{Event, KeepAlive, Sse},
    },
    routing::{delete, get, post},
};
use bollard::Docker;
use chrono::Utc;
use futures_util::{StreamExt, stream};
use serde::Deserialize;
use slasha_db::{
    DbPool,
    deployment::{Deployment, DeploymentStatus},
    models::app_scale::ProcessType,
    repos::{app::AppRepo, deployment::DeploymentRepo},
};
use tokio::sync::Notify;
use tokio_stream::wrappers::BroadcastStream;
use uuid::Uuid;

use crate::{
    docker::{
        deployment::{
            delete_deployment_processes, list_deployment_processes, restart_deployment_processes,
            run_deployment, scale_deployment_process, stop_deployment_processes,
        },
        logs::{LogKey, LogManager},
    },
    error::{HttpError, HttpResult},
    extractors::auth::AuthUser,
    state::{AppState, Runtime},
};

fn resolve_commit_message(repo_path: &str, sha: &str) -> anyhow::Result<String> {
    let repo = git2::Repository::open(repo_path)?;
    let commit = repo.find_commit(git2::Oid::from_str(sha)?)?;
    Ok(commit.summary().unwrap_or("").to_string())
}

fn resolve_head_commit(repo_path: &str, branch: &str) -> anyhow::Result<(String, String)> {
    let repo = git2::Repository::open(repo_path)?;
    let branch = repo.find_branch(branch, git2::BranchType::Local)?;
    let commit = branch.get().peel_to_commit()?;

    Ok((
        commit.id().to_string(),
        commit.summary().unwrap_or("").to_string(),
    ))
}

pub fn router() -> Router<AppState> {
    Router::new()
        .route("/", post(trigger_deploy).get(list_deployments))
        .route("/{deployment_id}", get(get_deployment))
        .route("/{deployment_id}/logs", get(stream_logs))
        .route("/{deployment_id}/stop", post(stop_deployment))
        .route("/{deployment_id}/restart", post(restart_deployment))
        .route("/{deployment_id}/redeploy", post(redeploy_deployment))
        .route("/{deployment_id}/scale", post(scale_deployment))
        .route("/{deployment_id}/processes", get(list_processes))
        .route("/{deployment_id}", delete(delete_deployment))
}

#[derive(Deserialize)]
struct TriggerDeployReq {
    commit_sha: Option<String>,
}

async fn trigger_deploy(
    State(docker): State<Docker>,
    State(db_pool): State<DbPool>,
    State(log_manager): State<Arc<LogManager>>,
    State(proxy_sync_trigger): State<Arc<Notify>>,
    AuthUser(user): AuthUser,
    Path(slug): Path<String>,
    Json(payload): Json<TriggerDeployReq>,
) -> HttpResult<impl IntoResponse> {
    let app = AppRepo::find_by_slug_for_user(&db_pool, &slug, &user.id).await?;

    let active_deployments = DeploymentRepo::list_active_for_app(&db_pool, &app.id).await?;
    let is_building = active_deployments
        .iter()
        .any(|d| d.status == DeploymentStatus::Building);

    if is_building {
        return Err(HttpError::bad_request(
            "A deployment is already building for this app",
        ));
    }

    let (commit_sha, commit_message) = match payload.commit_sha {
        Some(sha) => {
            let msg = resolve_commit_message(&app.repo_path, &sha)
                .map_err(|e| HttpError::bad_request(format!("Invalid commit SHA: {}", e)))?;
            (sha, msg)
        }
        None => resolve_head_commit(&app.repo_path, &app.default_branch).map_err(|e| {
            HttpError::bad_request(format!(
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

    let deployment = DeploymentRepo::create(&db_pool, deployment).await?;

    tokio::spawn(run_deployment(
        docker,
        db_pool,
        log_manager,
        proxy_sync_trigger,
        app,
        deployment.clone(),
    ));

    Ok(Json(serde_json::json!({ "deployment": deployment })))
}

async fn list_deployments(
    State(db_pool): State<DbPool>,
    AuthUser(user): AuthUser,
    Path(slug): Path<String>,
) -> HttpResult<impl IntoResponse> {
    let app = AppRepo::find_by_slug_for_user(&db_pool, &slug, &user.id).await?;

    let deps = DeploymentRepo::list_for_app(&db_pool, &app.id).await?;

    Ok(Json(serde_json::json!({ "deployments": deps })))
}

async fn get_deployment(
    State(db_pool): State<DbPool>,
    AuthUser(user): AuthUser,
    Path((slug, deployment_id)): Path<(String, String)>,
) -> HttpResult<impl IntoResponse> {
    let app = AppRepo::find_by_slug_for_user(&db_pool, &slug, &user.id).await?;

    let deployment = DeploymentRepo::find(&db_pool, &deployment_id, &app.id).await?;

    Ok(Json(serde_json::json!({ "deployment": deployment })))
}

async fn stop_deployment(
    State(docker): State<Docker>,
    State(db_pool): State<DbPool>,
    State(runtime): State<Runtime>,
    AuthUser(user): AuthUser,
    Path((slug, deployment_id)): Path<(String, String)>,
) -> HttpResult<impl IntoResponse> {
    let app = AppRepo::find_by_slug_for_user(&db_pool, &slug, &user.id).await?;

    let deployment = DeploymentRepo::find(&db_pool, &deployment_id, &app.id).await?;

    if !matches!(
        deployment.status,
        DeploymentStatus::Running | DeploymentStatus::Building
    ) {
        return Err(HttpError::bad_request(format!(
            "Deployment is already in state '{}'",
            deployment.status
        )));
    }

    stop_deployment_processes(
        &docker,
        &db_pool,
        &runtime.proxy_sync_trigger,
        &runtime.log_manager,
        &app,
        &deployment,
    )
    .await?;

    Ok(Json(serde_json::json!({
        "stopped": true,
        "deployment_id": deployment_id
    })))
}

async fn redeploy_deployment(
    State(docker): State<Docker>,
    State(db_pool): State<DbPool>,
    State(log_manager): State<Arc<LogManager>>,
    State(proxy_sync_trigger): State<Arc<Notify>>,
    AuthUser(user): AuthUser,
    Path((slug, deployment_id)): Path<(String, String)>,
) -> HttpResult<impl IntoResponse> {
    let app = AppRepo::find_by_slug_for_user(&db_pool, &slug, &user.id).await?;

    let deployment = DeploymentRepo::find(&db_pool, &deployment_id, &app.id).await?;

    delete_deployment_processes(
        &docker,
        &proxy_sync_trigger,
        &log_manager,
        &app,
        &deployment,
    )
    .await?;

    let now = Utc::now().naive_utc();
    let updated_deployment =
        DeploymentRepo::reset_to_pending(&db_pool, &deployment.id, now).await?;

    tokio::spawn(run_deployment(
        docker,
        db_pool,
        log_manager,
        proxy_sync_trigger,
        app,
        updated_deployment.clone(),
    ));

    Ok(Json(
        serde_json::json!({ "deployment": updated_deployment }),
    ))
}

async fn restart_deployment(
    State(docker): State<Docker>,
    State(db_pool): State<DbPool>,
    State(log_manager): State<Arc<LogManager>>,
    State(proxy_sync_trigger): State<Arc<Notify>>,
    AuthUser(user): AuthUser,
    Path((slug, deployment_id)): Path<(String, String)>,
) -> HttpResult<impl IntoResponse> {
    let app = AppRepo::find_by_slug_for_user(&db_pool, &slug, &user.id).await?;
    let deployment = DeploymentRepo::find(&db_pool, &deployment_id, &app.id).await?;

    restart_deployment_processes(
        &docker,
        &log_manager,
        &proxy_sync_trigger,
        &app,
        &deployment.id,
    )
    .await?;

    Ok(Json(serde_json::json!({
        "restarted": true,
        "deployment_id": deployment_id
    })))
}

async fn stream_logs(
    State(db_pool): State<DbPool>,
    State(log_manager): State<Arc<LogManager>>,
    AuthUser(user): AuthUser,
    Path((slug, deployment_id)): Path<(String, String)>,
) -> HttpResult<
    Sse<impl futures_util::Stream<Item = std::result::Result<Event, std::convert::Infallible>>>,
> {
    let app = AppRepo::find_by_slug_for_user(&db_pool, &slug, &user.id).await?;

    DeploymentRepo::find(&db_pool, &deployment_id, &app.id).await?;

    let log = log_manager
        .get_logger(&LogKey::Deployment {
            app_slug: app.slug.clone(),
            deployment_id,
        })
        .await
        .map_err(HttpError::internal)?;

    let historical = log.get_historical().await?;

    let historical_stream = stream::iter(
        historical
            .into_iter()
            .map(|msg| Ok(Event::default().data(msg))),
    );

    let rx = log.subscribe();
    let live_stream = BroadcastStream::new(rx).map(|res| match res {
        Ok(msg) => Ok(Event::default().data(msg)),
        Err(e) => Ok(Event::default().event("error").data(e.to_string())),
    });

    // marker to help distinguish between historical and live logs
    let done_marker = stream::once(async { Ok(Event::default().data("[done]")) });
    let combined = historical_stream.chain(done_marker).chain(live_stream);

    Ok(Sse::new(combined).keep_alive(KeepAlive::default()))
}

async fn delete_deployment(
    State(docker): State<Docker>,
    State(db_pool): State<DbPool>,
    State(log_manager): State<Arc<LogManager>>,
    State(proxy_sync_trigger): State<Arc<Notify>>,
    AuthUser(user): AuthUser,
    Path((slug, deployment_id)): Path<(String, String)>,
) -> HttpResult<impl IntoResponse> {
    let app = AppRepo::find_by_slug_for_user(&db_pool, &slug, &user.id).await?;

    let deployment = DeploymentRepo::find(&db_pool, &deployment_id, &app.id).await?;

    delete_deployment_processes(
        &docker,
        &proxy_sync_trigger,
        &log_manager,
        &app,
        &deployment,
    )
    .await?;

    DeploymentRepo::delete(&db_pool, &deployment.id, &app.id).await?;

    Ok(Json(serde_json::json!({
        "deleted": true,
        "deployment_id": deployment_id
    })))
}

#[derive(Deserialize)]
struct ScaleDeploymentReq {
    process_type: ProcessType,
    count: i32,
}

async fn scale_deployment(
    State(docker): State<Docker>,
    State(db_pool): State<DbPool>,
    State(log_manager): State<Arc<LogManager>>,
    State(proxy_sync_trigger): State<Arc<Notify>>,
    State(runtime): State<Runtime>,
    AuthUser(user): AuthUser,
    Path((slug, deployment_id)): Path<(String, String)>,
    Json(payload): Json<ScaleDeploymentReq>,
) -> HttpResult<impl IntoResponse> {
    if payload.count <= 0 {
        return Err(HttpError::bad_request("Count must be greater than 0"));
    }

    let app = AppRepo::find_by_slug_for_user(&db_pool, &slug, &user.id).await?;
    let deployment = DeploymentRepo::find(&db_pool, &deployment_id, &app.id).await?;

    if deployment.status != DeploymentStatus::Running {
        return Err(HttpError::bad_request(
            "Scaling is only allowed for running deployments",
        ));
    }

    let log_key = LogKey::Deployment {
        app_slug: app.slug.clone(),
        deployment_id: deployment.id.clone(),
    };
    let log = log_manager.get_logger(&log_key).await?;

    scale_deployment_process(
        &docker,
        &db_pool,
        &proxy_sync_trigger,
        &log,
        &app,
        &deployment,
        payload.process_type,
        payload.count as u32,
        runtime.get_scaling_lock(&deployment.id),
    )
    .await?;

    Ok(Json(serde_json::json!({
        "scaled": true,
        "process_type": payload.process_type,
        "count": payload.count
    })))
}

async fn list_processes(
    State(docker): State<Docker>,
    State(db_pool): State<DbPool>,
    AuthUser(user): AuthUser,
    Path((slug, deployment_id)): Path<(String, String)>,
) -> HttpResult<impl IntoResponse> {
    let app = AppRepo::find_by_slug_for_user(&db_pool, &slug, &user.id).await?;
    let deployment = DeploymentRepo::find(&db_pool, &deployment_id, &app.id).await?;

    let processes = list_deployment_processes(&docker, &deployment.id).await?;

    Ok(Json(serde_json::json!({ "processes": processes })))
}
