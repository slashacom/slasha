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
use garde::Validate;
use serde::Deserialize;
use slasha_db::{
    DbPool,
    app::AppSource,
    deployment::DeploymentStatus,
    models::app_scale::ProcessType,
    repos::{app_backup::AppBackupRepo, deployment::DeploymentRepo},
};
use tokio::sync::Notify;
use tokio_stream::wrappers::BroadcastStream;

use crate::{
    HttpError, HttpResult,
    connections::sync_external_app,
    docker::{
        deployment::{
            ScaleDeps, list_deployment_processes, remove_deployment_image,
            remove_deployment_processes, restart_deployment_processes, run_deployment,
            scale_deployment_process, stop_deployment_processes, trigger_deployment,
            trigger_rollback,
        },
        logs::{LogKey, LogManager},
    },
    extractors::{ValidatedJson, app::ActiveApp},
    state::{AppState, Runtime},
};

pub fn router() -> Router<AppState> {
    Router::new()
        .route("/", post(trigger_deploy).get(list_deployments))
        .route("/{deployment_id}", get(get_deployment))
        .route("/{deployment_id}/logs", get(stream_logs))
        .route("/{deployment_id}/stop", post(stop_deployment))
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
    ActiveApp { mut app, .. }: ActiveApp,
    ValidatedJson(payload): ValidatedJson<TriggerDeployReq>,
) -> HttpResult<impl IntoResponse> {
    if app.source != AppSource::Local {
        let github = state.github_client().await;
        sync_external_app(github.as_ref(), &state.storage, &state.runtime, &mut app)
            .await
            .map_err(|error| HttpError::bad_request(error.to_string()))?;
    }

    let deployment = trigger_deployment(
        state.clients.docker,
        state.storage.db_pool,
        state.runtime.log_manager,
        state.runtime.proxy_sync_trigger,
        state.runtime.deployment_tasks.clone(),
        app,
        payload.commit_sha,
    )
    .await
    .map_err(|e| HttpError::bad_request(format!("Failed to start deployment: {}", e)))?;

    match deployment {
        Some(deployment) => Ok(Json(serde_json::json!({ "deployment": deployment }))),
        None => Err(HttpError::bad_request(
            "A deployment is already building for this app",
        )),
    }
}

async fn list_deployments(
    State(db_pool): State<DbPool>,
    ActiveApp { app, .. }: ActiveApp,
) -> HttpResult<impl IntoResponse> {
    let deps = DeploymentRepo::list_for_app(&db_pool, &app.id).await?;

    Ok(Json(serde_json::json!({ "deployments": deps })))
}

async fn get_deployment(
    State(db_pool): State<DbPool>,
    ActiveApp { app, .. }: ActiveApp,
    Path((_, deployment_id)): Path<(String, String)>,
) -> HttpResult<impl IntoResponse> {
    let deployment = DeploymentRepo::find(&db_pool, &deployment_id, &app.id).await?;

    Ok(Json(serde_json::json!({ "deployment": deployment })))
}

async fn stop_deployment(
    State(docker_client): State<Docker>,
    State(db_pool): State<DbPool>,
    State(runtime): State<Runtime>,
    ActiveApp { app, .. }: ActiveApp,
    Path((_, deployment_id)): Path<(String, String)>,
) -> HttpResult<impl IntoResponse> {
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
        &docker_client,
        &db_pool,
        &runtime.proxy_sync_trigger,
        &runtime.log_manager,
        &runtime.deployment_tasks,
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
    State(docker_client): State<Docker>,
    State(db_pool): State<DbPool>,
    State(runtime): State<Runtime>,
    ActiveApp { app, .. }: ActiveApp,
    Path((_, deployment_id)): Path<(String, String)>,
) -> HttpResult<impl IntoResponse> {
    let active_deployments = DeploymentRepo::list_active_for_app(&db_pool, &app.id).await?;
    if active_deployments
        .iter()
        .any(|d| d.status == DeploymentStatus::Building)
    {
        return Err(HttpError::bad_request(
            "A deployment is already building for this app",
        ));
    }

    let deployment = DeploymentRepo::find(&db_pool, &deployment_id, &app.id).await?;

    remove_deployment_processes(
        &docker_client,
        &runtime.proxy_sync_trigger,
        &runtime.log_manager,
        &app,
        &deployment,
    )
    .await?;

    let now = Utc::now().naive_utc();
    let updated_deployment =
        DeploymentRepo::reset_to_pending(&db_pool, &deployment.id, now).await?;

    let handle = tokio::spawn(run_deployment(
        docker_client,
        db_pool,
        runtime.log_manager.clone(),
        runtime.proxy_sync_trigger.clone(),
        runtime.deployment_tasks.clone(),
        app,
        updated_deployment.clone(),
        None,
    ));

    runtime
        .deployment_tasks
        .insert(updated_deployment.id.clone(), handle.abort_handle());

    Ok(Json(
        serde_json::json!({ "deployment": updated_deployment }),
    ))
}

async fn restart_deployment(
    State(docker_client): State<Docker>,
    State(db_pool): State<DbPool>,
    State(log_manager): State<Arc<LogManager>>,
    State(proxy_sync_trigger): State<Arc<Notify>>,
    ActiveApp { app, .. }: ActiveApp,
    Path((_, deployment_id)): Path<(String, String)>,
) -> HttpResult<impl IntoResponse> {
    let active_deployments = DeploymentRepo::list_active_for_app(&db_pool, &app.id).await?;
    if active_deployments
        .iter()
        .any(|d| d.status == DeploymentStatus::Building)
    {
        return Err(HttpError::bad_request(
            "A deployment is already building for this app",
        ));
    }

    let deployment = DeploymentRepo::find(&db_pool, &deployment_id, &app.id).await?;

    restart_deployment_processes(
        &docker_client,
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

async fn rollback_deployment(
    State(docker_client): State<Docker>,
    State(db_pool): State<DbPool>,
    State(runtime): State<Runtime>,
    ActiveApp { app, .. }: ActiveApp,
    Path((_, deployment_id)): Path<(String, String)>,
) -> HttpResult<impl IntoResponse> {
    let source_deployment = DeploymentRepo::find(&db_pool, &deployment_id, &app.id).await?;

    let deployment = trigger_rollback(
        docker_client,
        db_pool,
        runtime.log_manager,
        runtime.proxy_sync_trigger,
        runtime.deployment_tasks,
        app,
        source_deployment,
    )
    .await
    .map_err(|error| HttpError::bad_request(format!("Failed to roll back: {}", error)))?
    .ok_or_else(|| HttpError::bad_request("A deployment is already building for this app"))?;

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

    // marker to help distinguish between historical and live logs
    let done_marker = stream::once(async { Ok(Event::default().data("[done]")) });
    let combined = historical_stream.chain(done_marker).chain(live_stream);

    Ok(Sse::new(combined).keep_alive(KeepAlive::default()))
}

async fn delete_deployment(
    State(docker_client): State<Docker>,
    State(db_pool): State<DbPool>,
    State(log_manager): State<Arc<LogManager>>,
    State(proxy_sync_trigger): State<Arc<Notify>>,
    ActiveApp { app, .. }: ActiveApp,
    Path((_, deployment_id)): Path<(String, String)>,
) -> HttpResult<impl IntoResponse> {
    let deployment = DeploymentRepo::find(&db_pool, &deployment_id, &app.id).await?;

    if matches!(
        deployment.status,
        DeploymentStatus::Running | DeploymentStatus::Building
    ) {
        return Err(HttpError::bad_request(
            "Active deployments must be stopped before deletion",
        ));
    }

    remove_deployment_processes(
        &docker_client,
        &proxy_sync_trigger,
        &log_manager,
        &app,
        &deployment,
    )
    .await?;

    remove_deployment_image(&docker_client, &app.slug, &deployment.id).await?;
    DeploymentRepo::delete(&db_pool, &deployment.id, &app.id).await?;

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
    count: i32,
}

async fn scale_deployment(
    State(app_state): State<AppState>,
    ActiveApp { app, .. }: ActiveApp,
    Path((_, deployment_id)): Path<(String, String)>,
    ValidatedJson(payload): ValidatedJson<ScaleDeploymentReq>,
) -> HttpResult<impl IntoResponse> {
    if payload.count <= 0 {
        return Err(HttpError::bad_request("Count must be greater than 0"));
    }

    let docker_client = app_state.clients.docker;
    let db_pool = app_state.storage.db_pool;
    let app_runtime = app_state.runtime;

    // litestream only allows one writer; multiple web instances would cause db corruption
    if payload.process_type == ProcessType::Web && payload.count > 1 {
        let backups_on = AppBackupRepo::get(&db_pool, &app.id)
            .await?
            .is_some_and(|b| b.enabled);
        if backups_on {
            return Err(HttpError::bad_request(
                "Backups require a single web instance (Litestream must be the only writer). Disable backups to scale web beyond 1.",
            ));
        }
    }

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

    let log = app_runtime.log_manager.get_logger(&log_key).await?;

    scale_deployment_process(
        ScaleDeps {
            docker_client: &docker_client,
            db_pool: &db_pool,
            proxy_sync: &app_runtime.proxy_sync_trigger,
            log: &log,
        },
        &app,
        &deployment,
        payload.process_type,
        payload.count as u32,
        app_runtime.get_scaling_lock(&deployment.id),
    )
    .await?;

    Ok(Json(serde_json::json!({
        "scaled": true,
        "process_type": payload.process_type,
        "count": payload.count
    })))
}

async fn list_processes(
    State(docker_client): State<Docker>,
    State(db_pool): State<DbPool>,
    ActiveApp { app, .. }: ActiveApp,
    Path((_, deployment_id)): Path<(String, String)>,
) -> HttpResult<impl IntoResponse> {
    let deployment = DeploymentRepo::find(&db_pool, &deployment_id, &app.id).await?;

    let processes = list_deployment_processes(&docker_client, &deployment.id).await?;

    Ok(Json(serde_json::json!({ "processes": processes })))
}
