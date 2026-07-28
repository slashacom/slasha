use std::sync::Arc;

use axum::{
    Json, Router,
    extract::{Path, State},
    http::StatusCode,
    response::{
        IntoResponse,
        sse::{Event, KeepAlive, Sse},
    },
    routing::{delete, get, post},
};
use futures_util::{StreamExt, stream};
use garde::Validate;
use serde::Deserialize;
use slasha_db::{DbPool, models::app_scale::ProcessType, repos::deployment::DeploymentRepo};
use tokio_stream::wrappers::BroadcastStream;

use crate::{
    HttpError, HttpResult,
    docker::AppDocker,
    extractors::{ValidatedJson, app::ActiveApp},
    logs::{LogKey, LogManager},
    state::AppState,
};

pub fn router() -> Router<AppState> {
    Router::new()
        .route("/", post(trigger_deploy))
        .route("/", get(list_deployments))
        .route("/{deployment_id}", get(get_deployment))
        .route("/{deployment_id}/logs", get(stream_logs))
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

async fn stream_logs(
    State(db_pool): State<DbPool>,
    State(log_manager): State<Arc<LogManager>>,
    ActiveApp { app, .. }: ActiveApp,
    Path((_, deployment_id)): Path<(String, String)>,
) -> HttpResult<
    Sse<impl futures_util::Stream<Item = std::result::Result<Event, std::convert::Infallible>>>,
> {
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

    let done_marker = stream::once(async { Ok(Event::default().data("[done]")) });
    let combined = historical_stream.chain(done_marker).chain(live_stream);

    Ok(Sse::new(combined).keep_alive(KeepAlive::default()))
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
