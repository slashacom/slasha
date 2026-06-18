use std::{collections::HashMap, sync::Arc};

use axum::{
    Json, Router,
    body::Body,
    extract::{Path, State, WebSocketUpgrade},
    http::header,
    response::{
        IntoResponse, Response,
        sse::{Event, KeepAlive, Sse},
    },
    routing::{delete, get, post},
};
use bollard::{
    Docker,
    exec::{CreateExecOptions, StartExecOptions, StartExecResults},
};
use chrono::Utc;
use futures_util::{StreamExt, stream};
use serde::Deserialize;
use slasha_db::{
    DbPool,
    repos::{app::AppRepo, service::ServiceRepo},
    service::{Service, ServiceKind, ServiceResources, ServiceStatus},
};
use tokio_stream::wrappers::BroadcastStream;
use uuid::Uuid;

use crate::{
    docker::{
        logs::{LogKey, LogManager},
        naming::service_container_name,
        service::{
            provision::resolve_env_vars, provision_service, remove_service_container,
            restart_service_container, stop_service_container,
        },
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
        .route("/{id}/backup", get(backup_service_handler))
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

const MIN_MEMORY_BYTES: i64 = 64 * 1024 * 1024;
const MIN_NANO_CPUS: i64 = 100_000_000;
const MIN_SHM_BYTES: i64 = 64 * 1024 * 1024;
const MIN_PIDS_LIMIT: i64 = 64;

async fn validate_resources(
    docker_client: &Docker,
    resources: &ServiceResources,
) -> HttpResult<()> {
    if let Some(mem) = resources.memory_bytes
        && mem < MIN_MEMORY_BYTES
    {
        return Err(HttpError::bad_request(format!(
            "memory must be at least {} MB",
            MIN_MEMORY_BYTES / (1024 * 1024)
        )));
    }
    if let Some(nc) = resources.nano_cpus
        && nc < MIN_NANO_CPUS
    {
        return Err(HttpError::bad_request("CPU must be at least 0.1 cores"));
    }
    if let Some(shm) = resources.shm_size
        && shm < MIN_SHM_BYTES
    {
        return Err(HttpError::bad_request(format!(
            "shared memory must be at least {} MB",
            MIN_SHM_BYTES / (1024 * 1024)
        )));
    }
    if let Some(pids) = resources.pids_limit
        && pids < MIN_PIDS_LIMIT
    {
        return Err(HttpError::bad_request(format!(
            "PID limit must be at least {}",
            MIN_PIDS_LIMIT
        )));
    }

    let info = docker_client
        .info()
        .await
        .map_err(|e| HttpError::internal(anyhow::anyhow!(e)))?;

    if let Some(host_mem) = info.mem_total
        && let Some(mem) = resources.memory_bytes
        && mem > host_mem
    {
        return Err(HttpError::bad_request(format!(
            "memory ({} MB) exceeds host capacity ({} MB)",
            mem / (1024 * 1024),
            host_mem / (1024 * 1024)
        )));
    }
    if let Some(host_cpus) = info.ncpu
        && let Some(nc) = resources.nano_cpus
    {
        let host_nano = host_cpus.saturating_mul(1_000_000_000);
        if nc > host_nano {
            return Err(HttpError::bad_request(format!(
                "CPU ({:.2} cores) exceeds host capacity ({} cores)",
                nc as f64 / 1_000_000_000.0,
                host_cpus
            )));
        }
    }
    if let Some(host_mem) = info.mem_total
        && let Some(shm) = resources.shm_size
        && shm > host_mem
    {
        return Err(HttpError::bad_request(format!(
            "shared memory ({} MB) exceeds host capacity ({} MB)",
            shm / (1024 * 1024),
            host_mem / (1024 * 1024)
        )));
    }

    Ok(())
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
    State(docker_client): State<Docker>,
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

    if let Some(ref resources) = payload.resources {
        validate_resources(&docker_client, resources).await?;
    }

    let now = Utc::now().naive_utc();
    let service_id = Uuid::new_v4().to_string();

    let new_service = Service {
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
        docker_client,
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
    State(docker_client): State<Docker>,
    State(db_pool): State<DbPool>,
    AuthUser(user): AuthUser,
    Path((slug, id)): Path<(String, String)>,
) -> HttpResult<impl IntoResponse> {
    let app = AppRepo::find_by_slug_for_user(&db_pool, &slug, &user.id).await?;
    let service = ServiceRepo::find(&db_pool, &id, &app.id).await?;

    if service.status != ServiceStatus::Running {
        return Err(HttpError::bad_request("Service is not running"));
    }

    let user_id = user.id.clone();
    Ok(ws.on_upgrade(move |socket| async move {
        tunnel::handle_tunnel(socket, docker_client, db_pool, service, user_id).await;
    }))
}

async fn restart_service_handler(
    State(docker_client): State<Docker>,
    State(db_pool): State<DbPool>,
    State(log_manager): State<Arc<LogManager>>,
    AuthUser(user): AuthUser,
    Path((slug, id)): Path<(String, String)>,
) -> HttpResult<impl IntoResponse> {
    let app = AppRepo::find_by_slug_for_user(&db_pool, &slug, &user.id).await?;
    let service = ServiceRepo::find(&db_pool, &id, &app.id).await?;

    restart_service_container(&docker_client, &db_pool, &log_manager, &app, &service).await?;

    Ok(Json(serde_json::json!({ "restarted": true })))
}

async fn redeploy_service_handler(
    State(docker_client): State<Docker>,
    State(db_pool): State<DbPool>,
    State(log_manager): State<Arc<LogManager>>,
    AuthUser(user): AuthUser,
    Path((slug, id)): Path<(String, String)>,
) -> HttpResult<impl IntoResponse> {
    let app = AppRepo::find_by_slug_for_user(&db_pool, &slug, &user.id).await?;
    let service = ServiceRepo::find(&db_pool, &id, &app.id).await?;

    remove_service_container(&docker_client, &log_manager, &app, &service, false).await?;

    tokio::spawn(provision_service(
        docker_client,
        db_pool,
        log_manager,
        app,
        service,
        None,
    ));

    Ok(Json(serde_json::json!({ "redeploying": true })))
}

async fn stop_service_handler(
    State(docker_client): State<Docker>,
    State(db_pool): State<DbPool>,
    State(log_manager): State<Arc<LogManager>>,
    AuthUser(user): AuthUser,
    Path((slug, id)): Path<(String, String)>,
) -> HttpResult<impl IntoResponse> {
    let app = AppRepo::find_by_slug_for_user(&db_pool, &slug, &user.id).await?;
    let service = ServiceRepo::find(&db_pool, &id, &app.id).await?;

    if service.status != ServiceStatus::Running {
        return Err(HttpError::bad_request("Service is not running"));
    }

    stop_service_container(&docker_client, &db_pool, &log_manager, &app, &service).await?;

    Ok(Json(serde_json::json!({ "stopped": true })))
}

async fn delete_service_handler(
    State(docker_client): State<Docker>,
    State(db_pool): State<DbPool>,
    State(log_manager): State<Arc<LogManager>>,
    AuthUser(user): AuthUser,
    Path((slug, id)): Path<(String, String)>,
) -> HttpResult<impl IntoResponse> {
    let app = AppRepo::find_by_slug_for_user(&db_pool, &slug, &user.id).await?;
    let service = ServiceRepo::find(&db_pool, &id, &app.id).await?;

    if service.status != ServiceStatus::Stopped && service.status != ServiceStatus::Failed {
        return Err(HttpError::bad_request(
            "Cannot delete a running or provisioning service. Please stop it first.",
        ));
    }

    remove_service_container(&docker_client, &log_manager, &app, &service, true).await?;

    ServiceRepo::delete(&db_pool, &service.id).await?;

    Ok(Json(serde_json::json!({ "deleted": true })))
}

async fn backup_service_handler(
    State(docker_client): State<Docker>,
    State(db_pool): State<DbPool>,
    AuthUser(user): AuthUser,
    Path((slug, id)): Path<(String, String)>,
) -> HttpResult<impl IntoResponse> {
    let app = AppRepo::find_by_slug_for_user(&db_pool, &slug, &user.id).await?;
    let service = ServiceRepo::find(&db_pool, &id, &app.id).await?;

    if service.status != ServiceStatus::Running {
        return Err(HttpError::bad_request("Service is not running"));
    }

    let env_vars = ServiceRepo::get_env_vars(&db_pool, &service.id).await?;
    let resolved = resolve_env_vars(env_vars, &service)?;

    let cmd = service.kind.backup_cmd(&resolved);
    let container_name = service_container_name(&service.id);

    let exec_id = docker_client
        .create_exec(
            &container_name,
            CreateExecOptions {
                attach_stdout: Some(true),
                attach_stderr: Some(false),
                cmd: Some(cmd),
                ..Default::default()
            },
        )
        .await
        .map_err(|e| HttpError::internal(anyhow::anyhow!(e)))?;

    let output_stream = match docker_client
        .start_exec(&exec_id.id, None::<StartExecOptions>)
        .await
        .map_err(|e| HttpError::internal(anyhow::anyhow!(e)))?
    {
        StartExecResults::Attached { output, .. } => output,
        StartExecResults::Detached => {
            return Err(HttpError::internal(anyhow::anyhow!(
                "exec returned detached"
            )));
        }
    };

    let byte_stream = output_stream.filter_map(|item| async move {
        match item {
            Ok(bollard::container::LogOutput::StdOut { message }) => {
                Some(Ok::<_, std::io::Error>(message))
            }
            _ => None,
        }
    });

    let timestamp = Utc::now().format("%Y%m%d%H%M%S");
    let filename = format!("{}-{}.dump", service.name, timestamp);

    let response = Response::builder()
        .header(header::CONTENT_TYPE, "application/octet-stream")
        .header(
            header::CONTENT_DISPOSITION,
            format!("attachment; filename=\"{}\"", filename),
        )
        .body(Body::from_stream(byte_stream))
        .map_err(|e| HttpError::internal(anyhow::anyhow!(e)))?;

    Ok(response)
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
    let service = ServiceRepo::find(&db_pool, &id, &app.id).await?;

    let log = log_manager
        .get_logger(&LogKey::Service {
            app_slug: app.slug.clone(),
            service_name: service.name.clone(),
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
