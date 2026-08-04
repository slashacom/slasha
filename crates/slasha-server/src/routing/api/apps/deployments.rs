use axum::{
    Json, Router,
    extract::{Path, Query, State},
    http::StatusCode,
    response::IntoResponse,
    routing::{delete, get, post},
};
use garde::Validate;
use serde::Deserialize;
use slasha_db::{
    DbPool, DuckdbPool, models::app_scale::ProcessType, repos::deployment::DeploymentRepo,
};

use crate::{
    HttpResult,
    docker::AppDocker,
    extractors::{ValidatedJson, app::ActiveApp},
    logs::LogBus,
    routing::api::logs::{LogQuery, fetch_resource_logs, stream_resource_logs},
    state::AppState,
};

pub fn router() -> Router<AppState> {
    Router::new()
        .route("/", post(trigger_deploy))
        .route("/", get(list_deployments))
        .route("/{deployment_id}", get(get_deployment))
        .route("/{deployment_id}/logs", get(get_logs))
        .route("/{deployment_id}/stream", get(stream_logs))
        .route("/{deployment_id}/stop", post(stop_deployment))
        .route("/{deployment_id}/cancel", post(cancel_deployment))
        .route("/{deployment_id}/restart", post(restart_deployment))
        .route("/{deployment_id}/redeploy", post(redeploy_deployment))
        .route("/{deployment_id}/rollback", post(rollback_deployment))
        .route("/{deployment_id}/scale", post(scale_deployment))
        .route("/{deployment_id}/processes", get(list_processes))
        .route("/{deployment_id}", delete(delete_deployment))
}

#[derive(Deserialize, Validate)]
struct TriggerDeployReq {
    #[garde(skip)]
    commit_sha: Option<String>,
}

async fn trigger_deploy(
    State(state): State<AppState>,
    ActiveApp { app, .. }: ActiveApp,
    ValidatedJson(payload): ValidatedJson<TriggerDeployReq>,
) -> HttpResult<impl IntoResponse> {
    let deployment = AppDocker::new(state, app)
        .await?
        .deploy(payload.commit_sha)
        .await?;

    Ok(Json(serde_json::json!({ "deployment": deployment })))
}

async fn list_deployments(
    State(db_pool): State<DbPool>,
    ActiveApp { app, .. }: ActiveApp,
) -> HttpResult<impl IntoResponse> {
    let deployments = DeploymentRepo::list_for_app(&db_pool, &app.id).await?;

    Ok(Json(serde_json::json!({ "deployments": deployments })))
}

async fn get_deployment(
    State(db_pool): State<DbPool>,
    ActiveApp { app, .. }: ActiveApp,
    Path((_, deployment_id)): Path<(String, String)>,
) -> HttpResult<impl IntoResponse> {
    let deployment = DeploymentRepo::find(&db_pool, &deployment_id, &app.id).await?;

    Ok(Json(serde_json::json!({ "deployment": deployment })))
}

async fn cancel_deployment(
    State(state): State<AppState>,
    ActiveApp { app, .. }: ActiveApp,
    Path((_, deployment_id)): Path<(String, String)>,
) -> HttpResult<impl IntoResponse> {
    AppDocker::new(state, app)
        .await?
        .cancel_deployment(&deployment_id)
        .await?;

    Ok(Json(serde_json::json!({
        "cancelled": true,
        "deployment_id": deployment_id
    })))
}

async fn stop_deployment(
    State(state): State<AppState>,
    ActiveApp { app, .. }: ActiveApp,
    Path((_, deployment_id)): Path<(String, String)>,
) -> HttpResult<impl IntoResponse> {
    AppDocker::new(state, app)
        .await?
        .stop_deployment(&deployment_id)
        .await?;

    Ok(Json(serde_json::json!({
        "stopped": true,
        "deployment_id": deployment_id
    })))
}

async fn redeploy_deployment(
    State(state): State<AppState>,
    ActiveApp { app, .. }: ActiveApp,
    Path((_, deployment_id)): Path<(String, String)>,
) -> HttpResult<impl IntoResponse> {
    let updated_deployment = AppDocker::new(state, app)
        .await?
        .redeploy(&deployment_id)
        .await?;

    Ok(Json(
        serde_json::json!({ "deployment": updated_deployment }),
    ))
}

async fn restart_deployment(
    State(state): State<AppState>,
    ActiveApp { app, .. }: ActiveApp,
    Path((_, deployment_id)): Path<(String, String)>,
) -> HttpResult<impl IntoResponse> {
    AppDocker::new(state, app)
        .await?
        .restart_deployment(&deployment_id)
        .await?;

    Ok(Json(serde_json::json!({
        "restarted": true,
        "deployment_id": deployment_id
    })))
}

async fn rollback_deployment(
    State(state): State<AppState>,
    ActiveApp { app, .. }: ActiveApp,
    Path((_, deployment_id)): Path<(String, String)>,
) -> HttpResult<impl IntoResponse> {
    let deployment = AppDocker::new(state, app)
        .await?
        .rollback_to_deployment(&deployment_id)
        .await?;

    Ok(Json(serde_json::json!({ "deployment": deployment })))
}

async fn get_logs(
    State(db_pool): State<DbPool>,
    State(duckdb_pool): State<DuckdbPool>,
    ActiveApp { app, .. }: ActiveApp,
    Path((_, deployment_id)): Path<(String, String)>,
    Query(query): Query<LogQuery>,
) -> HttpResult<impl IntoResponse> {
    DeploymentRepo::find(&db_pool, &deployment_id, &app.id).await?;
    fetch_resource_logs(&duckdb_pool, &deployment_id, query).await
}

async fn stream_logs(
    State(db_pool): State<DbPool>,
    State(log_bus): State<LogBus>,
    ActiveApp { app, .. }: ActiveApp,
    Path((_, deployment_id)): Path<(String, String)>,
) -> HttpResult<impl IntoResponse> {
    DeploymentRepo::find(&db_pool, &deployment_id, &app.id).await?;
    stream_resource_logs(&log_bus, &deployment_id).await
}

async fn delete_deployment(
    State(state): State<AppState>,
    ActiveApp { app, .. }: ActiveApp,
    Path((_, deployment_id)): Path<(String, String)>,
) -> HttpResult<impl IntoResponse> {
    AppDocker::new(state, app)
        .await?
        .delete_deployment(&deployment_id)
        .await?;

    Ok(Json(serde_json::json!({
        "deleted": true,
        "deployment_id": deployment_id
    })))
}

#[derive(Deserialize, Validate)]
struct ScaleDeploymentReq {
    #[garde(skip)]
    process_type: ProcessType,
    #[garde(range(min = 1))]
    count: u32,
}

async fn scale_deployment(
    State(app_state): State<AppState>,
    ActiveApp { app, .. }: ActiveApp,
    Path((_, deployment_id)): Path<(String, String)>,
    ValidatedJson(payload): ValidatedJson<ScaleDeploymentReq>,
) -> HttpResult<impl IntoResponse> {
    AppDocker::new(app_state, app)
        .await?
        .scale(&deployment_id, payload.process_type, payload.count)
        .await?;

    Ok(StatusCode::NO_CONTENT)
}

async fn list_processes(
    State(state): State<AppState>,
    ActiveApp { app, .. }: ActiveApp,
    Path((_, deployment_id)): Path<(String, String)>,
) -> HttpResult<impl IntoResponse> {
    let processes = AppDocker::new(state, app)
        .await?
        .list_processes(&deployment_id)
        .await?;

    Ok(Json(serde_json::json!({ "processes": processes })))
}
