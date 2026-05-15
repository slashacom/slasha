use std::{collections::HashMap, sync::Arc};

use axum::{
    Json, Router,
    extract::{Path, State, WebSocketUpgrade},
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
    repos::{app::AppRepo, service::ServiceRepo},
    service::{ServiceKind, ServiceResources, ServiceStatus},
};
use tokio::sync::Notify;
use tokio_stream::wrappers::BroadcastStream;
use uuid::Uuid;

use crate::{
    docker::{
        logs::{LogKey, LogManager},
        naming::service_container_name,
        service::{delete_service, provision_service, restart_service, stop_service},
    },
    error::{HttpError, HttpResult},
    extractors::auth::AuthUser,
    state::AppState,
    tunnel,
};

pub fn router() -> Router<AppState> {
    Router::new()
        .route("/", get(list_services))
        .route("/", post(create_service))
        .route("/{id}/logs", get(stream_logs))
        .route("/{id}/tunnel", get(tunnel_handler))
        .route("/{id}/restart", post(restart_service_handler))
        .route("/{id}/redeploy", post(redeploy_service_handler))
        .route("/{id}/stop", post(stop_service_handler))
        .route("/{id}", delete(delete_service_handler))
}

#[derive(Deserialize)]
struct CreateServiceReq {
    kind: ServiceKind,
    name: String,
    version: String,
    env_vars: HashMap<String, String>,
    #[serde(default)]
    resources: Option<ServiceResources>,
}

async fn list_services(
    State(db_pool): State<DbPool>,
    AuthUser(user): AuthUser,
    Path(slug): Path<String>,
) -> HttpResult<impl IntoResponse> {
    let app = AppRepo::find_by_slug_for_user(&db_pool, &slug, &user.id).await?;
    let services = ServiceRepo::list_for_app(&db_pool, &app.id).await?;

    Ok(Json(serde_json::json!({
        "services": services,
    })))
}

async fn create_service(
    State(docker): State<Docker>,
    State(db_pool): State<DbPool>,
    State(log_manager): State<Arc<LogManager>>,
    AuthUser(user): AuthUser,
    Path(slug): Path<String>,
    Json(payload): Json<CreateServiceReq>,
) -> HttpResult<impl IntoResponse> {
    let app = AppRepo::find_by_slug_for_user(&db_pool, &slug, &user.id).await?;

    if !payload
        .kind
        .supported_versions()
        .contains(&payload.version.as_str())
    {
        return Err(HttpError::bad_request(format!(
            "Version {} is not supported for {:?}. Supported versions: {:?}",
            payload.version,
            payload.kind,
            payload.kind.supported_versions()
        )));
    }

    for key in payload.kind.secret_env_keys() {
        let missing = payload
            .env_vars
            .get(*key)
            .map(|v| v.trim().is_empty())
            .unwrap_or(true);
        if missing {
            return Err(HttpError::bad_request(format!(
                "{} is required and cannot be empty",
                key
            )));
        }
    }

    let now = Utc::now().naive_utc();
    let service_id = Uuid::new_v4().to_string();

    let new_service = slasha_db::service::Service {
        id: service_id.clone(),
        app_id: app.id.clone(),
        kind: payload.kind,
        name: payload.name.trim().to_string(),
        version: payload.version,
        status: ServiceStatus::Provisioning,
        created_at: now,
        updated_at: now,
        resources: payload.resources,
    };

    let new_service = ServiceRepo::create(&db_pool, new_service).await?;

    tokio::spawn(provision_service(
        docker,
        db_pool,
        log_manager,
        app,
        new_service.clone(),
        Some(payload.env_vars),
    ));

    Ok(Json(serde_json::json!({
        "service": new_service,
    })))
}

async fn tunnel_handler(
    ws: WebSocketUpgrade,
    State(docker): State<Docker>,
    State(db_pool): State<DbPool>,
    AuthUser(user): AuthUser,
    Path((slug, id)): Path<(String, String)>,
) -> HttpResult<impl IntoResponse> {
    let app = AppRepo::find_by_slug_for_user(&db_pool, &slug, &user.id).await?;
    let svc = ServiceRepo::find(&db_pool, &id, &app.id).await?;

    if svc.status != ServiceStatus::Running {
        return Err(HttpError::bad_request("Service is not running"));
    }

    let user_id = user.id.clone();
    Ok(ws.on_upgrade(move |socket| async move {
        tunnel::handle_tunnel(socket, docker, svc, user_id).await;
    }))
}

async fn restart_service_handler(
    State(docker): State<Docker>,
    State(db_pool): State<DbPool>,
    State(log_manager): State<Arc<LogManager>>,
    State(proxy_sync_trigger): State<Arc<Notify>>,
    AuthUser(user): AuthUser,
    Path((slug, id)): Path<(String, String)>,
) -> HttpResult<impl IntoResponse> {
    let app = AppRepo::find_by_slug_for_user(&db_pool, &slug, &user.id).await?;
    let svc = ServiceRepo::find(&db_pool, &id, &app.id).await?;

    restart_service(
        &docker,
        &db_pool,
        &log_manager,
        &proxy_sync_trigger,
        &app,
        &svc,
    )
    .await?;

    Ok(Json(serde_json::json!({ "restarted": true })))
}

async fn redeploy_service_handler(
    State(docker): State<Docker>,
    State(db_pool): State<DbPool>,
    State(log_manager): State<Arc<LogManager>>,
    AuthUser(user): AuthUser,
    Path((slug, id)): Path<(String, String)>,
) -> HttpResult<impl IntoResponse> {
    let app = AppRepo::find_by_slug_for_user(&db_pool, &slug, &user.id).await?;
    let svc = ServiceRepo::find(&db_pool, &id, &app.id).await?;

    let container_name = service_container_name(&svc.id);
    let _ = docker
        .remove_container(
            &container_name,
            Some(
                bollard::query_parameters::RemoveContainerOptionsBuilder::new()
                    .force(true)
                    .build(),
            ),
        )
        .await;

    log_manager.remove(&LogKey::Service {
        app_slug: slug,
        service_name: svc.name.clone(),
    });

    ServiceRepo::update_status(&db_pool, &svc.id, ServiceStatus::Provisioning).await?;

    tokio::spawn(provision_service(
        docker,
        db_pool,
        log_manager,
        app,
        svc,
        None,
    ));

    Ok(Json(serde_json::json!({ "redeploying": true })))
}

async fn stop_service_handler(
    State(docker): State<Docker>,
    State(db_pool): State<DbPool>,
    State(log_manager): State<Arc<LogManager>>,
    AuthUser(user): AuthUser,
    Path((slug, id)): Path<(String, String)>,
) -> HttpResult<impl IntoResponse> {
    let app = AppRepo::find_by_slug_for_user(&db_pool, &slug, &user.id).await?;
    let svc = ServiceRepo::find(&db_pool, &id, &app.id).await?;

    if svc.status != ServiceStatus::Running {
        return Err(HttpError::bad_request("Service is not running"));
    }

    stop_service(&docker, &db_pool, &log_manager, &app, &svc).await?;

    Ok(Json(serde_json::json!({ "stopped": true })))
}

async fn delete_service_handler(
    State(docker): State<Docker>,
    State(db_pool): State<DbPool>,
    State(log_manager): State<Arc<LogManager>>,
    AuthUser(user): AuthUser,
    Path((slug, id)): Path<(String, String)>,
) -> HttpResult<impl IntoResponse> {
    let app = AppRepo::find_by_slug_for_user(&db_pool, &slug, &user.id).await?;
    let svc = ServiceRepo::find(&db_pool, &id, &app.id).await?;

    if svc.status != ServiceStatus::Stopped && svc.status != ServiceStatus::Failed {
        return Err(HttpError::bad_request(
            "Cannot delete a running or provisioning service. Please stop it first.",
        ));
    }

    delete_service(&docker, &db_pool, &log_manager, &app, &svc).await?;

    Ok(Json(serde_json::json!({ "deleted": true })))
}

async fn stream_logs(
    State(db_pool): State<DbPool>,
    State(log_manager): State<Arc<LogManager>>,
    AuthUser(user): AuthUser,
    Path((slug, id)): Path<(String, String)>,
) -> HttpResult<
    Sse<impl futures_util::Stream<Item = std::result::Result<Event, std::convert::Infallible>>>,
> {
    let app = AppRepo::find_by_slug_for_user(&db_pool, &slug, &user.id).await?;
    let svc = ServiceRepo::find(&db_pool, &id, &app.id).await?;

    let log = log_manager
        .get_logger(&LogKey::Service {
            app_slug: app.slug.clone(),
            service_name: svc.name.clone(),
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
