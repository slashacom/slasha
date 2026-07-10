use std::sync::Arc;

use axum::{
    Json, Router,
    extract::{Path, Query, State},
    response::{
        IntoResponse,
        sse::{Event, KeepAlive, Sse},
    },
    routing::{delete, get, post, put},
};
use futures_util::{StreamExt, stream};
use garde::Validate;
use serde::{Deserialize, Serialize};
use slasha_db::{
    DbPool,
    models::node::{NewNode, Node, NodeChangeset, NodeStatus},
    repos::node::NodeRepo,
};
use tokio_stream::wrappers::BroadcastStream;
use uuid::Uuid;

use crate::{
    HttpError, HttpResult,
    docker::registry::DockerRegistry,
    extractors::{ValidatedJson, auth::AuthUser},
    logs::{LogKey, LogManager},
    routing::api::validation::not_empty,
    state::AppState,
};

pub fn router() -> Router<AppState> {
    Router::new()
        .route("/", get(list_nodes))
        .route("/", post(create_node))
        .route("/{id}", get(get_node))
        .route("/{id}", put(update_node))
        .route("/{id}", delete(delete_node))
        .route("/{id}/logs", get(stream_node_logs))
}

#[derive(Serialize)]
struct NodeWithStatus {
    #[serde(flatten)]
    node: Node,
    live_status: String,
}

async fn resolve_status(registry: &DockerRegistry, node: &Node) -> String {
    if let NodeStatus::Ready = node.status {
        if let Ok(docker) = registry.get_client(node)
            && docker.ping().await.is_ok()
        {
            "online".to_string()
        } else {
            "offline".to_string()
        }
    } else {
        node.status.to_string()
    }
}

async fn list_nodes(
    State(state): State<AppState>,
    AuthUser(_user): AuthUser,
) -> HttpResult<impl IntoResponse> {
    let nodes = NodeRepo::list(&state.storage.db_pool).await?;
    let mut results = Vec::new();

    for node in nodes {
        let live_status = resolve_status(&state.clients.docker_registry, &node).await;
        results.push(NodeWithStatus { node, live_status });
    }

    Ok(Json(serde_json::json!({ "nodes": results })))
}

async fn get_node(
    State(state): State<AppState>,
    AuthUser(_user): AuthUser,
    Path(id): Path<String>,
) -> HttpResult<impl IntoResponse> {
    let node = NodeRepo::get(&state.storage.db_pool, &id).await?;
    let live_status = resolve_status(&state.clients.docker_registry, &node).await;

    Ok(Json(serde_json::json!({
        "node": NodeWithStatus { node, live_status }
    })))
}

#[derive(Deserialize, Validate)]
struct CreateNodeReq {
    #[garde(custom(not_empty))]
    name: String,
    #[garde(custom(not_empty))]
    host: String,
    #[garde(custom(not_empty))]
    user: String,
    #[garde(range(min = 1, max = 65535))]
    port: Option<i32>,
    #[garde(custom(not_empty))]
    ssh_private_key: String,
}

const SETUP_SCRIPT: &str = include_str!("setup.sh");
const TEARDOWN_SCRIPT: &str = include_str!("teardown.sh");

async fn create_node(
    State(state): State<AppState>,
    AuthUser(_user): AuthUser,
    ValidatedJson(payload): ValidatedJson<CreateNodeReq>,
) -> HttpResult<impl IntoResponse> {
    let port = payload.port.unwrap_or(22);

    let new_node = NewNode {
        id: Uuid::new_v4().to_string(),
        name: payload.name,
        host: Some(payload.host.clone()),
        user: Some(payload.user.clone()),
        port: Some(port),
        ssh_private_key: Some(payload.ssh_private_key.clone()),
        internal_root_ca: None,
        status: NodeStatus::SettingUp,
    };

    // the node hasn't been inserted into the database yet
    // create a temporary node so we can reuse the ssh probe logic
    if let Err(e) = state
        .clients
        .node_connection_manager
        .probe_ssh(&Node {
            id: new_node.id.clone(),
            name: new_node.name.clone(),
            host: new_node.host.clone(),
            user: new_node.user.clone(),
            port: new_node.port,
            ssh_private_key: new_node.ssh_private_key.clone(),
            internal_root_ca: None,
            status: NodeStatus::SettingUp,
            created_at: chrono::Utc::now().naive_utc(),
            updated_at: chrono::Utc::now().naive_utc(),
            deleted_at: None,
        })
        .await
    {
        return Err(HttpError::bad_request(e.to_string()));
    }

    let node = NodeRepo::create(&state.storage.db_pool, new_node).await?;

    let setup_script = format!("export SSH_PORT={}\n{}", port, SETUP_SCRIPT);

    let log_handle = state
        .runtime
        .log_manager
        .get_logger(&LogKey::NodeSetup {
            node_id: node.id.clone(),
        })
        .await
        .map_err(HttpError::internal)?;

    tokio::spawn({
        let db_pool = state.storage.db_pool.clone();
        let node_connection_manager = state.clients.node_connection_manager.clone();
        let node = node.clone();

        async move {
            let result = node_connection_manager
                .run_ssh_script_streaming(&node, &setup_script, &log_handle)
                .await;

            let mut internal_root_ca = None;
            let status = match result {
                Ok(stdout_str) => {
                    if let Some(start) = stdout_str.find("---BEGIN ROOT CA---\n")
                        && let Some(end) = stdout_str[start..].find("\n---END ROOT CA---")
                    {
                        let cert = stdout_str[start + "---BEGIN ROOT CA---\n".len()..start + end]
                            .trim()
                            .to_string();
                        if !cert.is_empty() {
                            internal_root_ca = Some(cert);
                        }
                    }
                    tracing::info!(node_id = %node.id, node_name = %node.name, "node setup completed");
                    let _ = log_handle.send("setup completed successfully").await;
                    NodeStatus::Ready
                }
                Err(e) => {
                    tracing::error!(node_id = %node.id, node_name = %node.name, error = %e, "node setup failed");
                    let _ = log_handle.send(format!("setup failed: {}", e)).await;
                    NodeStatus::Error
                }
            };

            if let Err(e) =
                NodeRepo::set_status_and_ca(&db_pool, &node.id, status, internal_root_ca).await
            {
                tracing::error!(node_id = %node.id, node_name = %node.name, error = %e, "failed to update node status");
            }
        }
    });

    Ok(Json(serde_json::json!({ "node": node })))
}

#[derive(Deserialize, Validate)]
struct UpdateNodeReq {
    #[garde(inner(custom(not_empty)))]
    name: Option<String>,
    #[garde(inner(custom(not_empty)))]
    host: Option<String>,
    #[garde(inner(custom(not_empty)))]
    user: Option<String>,
    #[garde(skip)]
    port: Option<i32>,
    #[garde(inner(custom(not_empty)))]
    ssh_private_key: Option<String>,
}

async fn update_node(
    State(state): State<AppState>,
    AuthUser(_user): AuthUser,
    Path(id): Path<String>,
    ValidatedJson(payload): ValidatedJson<UpdateNodeReq>,
) -> HttpResult<impl IntoResponse> {
    let mut node = NodeRepo::get(&state.storage.db_pool, &id).await?;

    let connection_changed = payload.host.is_some()
        || payload.user.is_some()
        || payload.port.is_some()
        || payload.ssh_private_key.is_some();

    if node.is_local() && connection_changed {
        return Err(HttpError::bad_request(
            "can only update 'name' for the local node",
        ));
    }

    let changeset = NodeChangeset {
        name: payload.name.clone(),
        host: payload.host.clone().map(Some),
        user: payload.user.clone().map(Some),
        port: payload.port.map(Some),
        ssh_private_key: payload.ssh_private_key.clone().map(Some),
        internal_root_ca: None,
        status: None,
    };

    if connection_changed {
        node.host = payload.host.clone().or(node.host);
        node.user = payload.user.clone().or(node.user);
        node.port = payload.port.or(node.port);
        node.ssh_private_key = payload.ssh_private_key.clone().or(node.ssh_private_key);

        state
            .clients
            .node_connection_manager
            .probe_ssh(&node)
            .await
            .map_err(|e| HttpError::bad_request(e.to_string()))?;
    }

    let node = NodeRepo::update(&state.storage.db_pool, &id, changeset).await?;
    state.clients.docker_registry.remove(&id);

    Ok(Json(serde_json::json!({ "node": node })))
}

async fn delete_node(
    State(state): State<AppState>,
    AuthUser(_user): AuthUser,
    Path(id): Path<String>,
) -> HttpResult<impl IntoResponse> {
    let node = NodeRepo::get(&state.storage.db_pool, &id).await?;

    if node.is_local() {
        return Err(HttpError::bad_request("cannot delete the local node"));
    }

    let has_apps = NodeRepo::has_apps(&state.storage.db_pool, &id).await?;
    if has_apps {
        return Err(HttpError::bad_request(
            "cannot delete node while it has apps assigned",
        ));
    }

    NodeRepo::set_status(&state.storage.db_pool, &id, NodeStatus::Deleting).await?;

    tokio::spawn({
        let db_pool = state.storage.db_pool.clone();
        let node_connection_manager = state.clients.node_connection_manager.clone();
        let docker_registry = state.clients.docker_registry.clone();
        let node = node.clone();
        let log_manager = state.runtime.log_manager.clone();
        let log_handle = log_manager
            .get_logger(&LogKey::NodeTeardown {
                node_id: id.clone(),
            })
            .await?;

        async move {
            let result = node_connection_manager
                .run_ssh_script_streaming(&node, TEARDOWN_SCRIPT, &log_handle)
                .await;

            match result {
                Ok(_) => {
                    tracing::info!(node_id = %node.id, node_name = %node.name, "node teardown completed");
                    let _ = log_handle.send("teardown completed successfully").await;

                    if let Err(e) = NodeRepo::delete(&db_pool, &node.id).await {
                        tracing::error!(node_id = %node.id, error = %e, "failed to delete node from database");
                    } else {
                        node_connection_manager.remove_key(&node.id);
                        let _ = log_manager.delete_node_logs(&node.id).await;
                    }
                }
                Err(e) => {
                    tracing::error!(node_id = %node.id, error = %e, "node teardown failed");
                    let _ = log_handle.send(format!("teardown failed: {}", e)).await;

                    if let Err(db_err) =
                        NodeRepo::set_status(&db_pool, &node.id, NodeStatus::Error).await
                    {
                        tracing::error!(node_id = %node.id, error = %db_err, "failed to set node status to error");
                    }
                }
            }

            docker_registry.remove(&node.id);
        }
    });

    Ok(Json(
        serde_json::json!({ "deleting": true, "deleted": true }),
    ))
}

#[derive(Deserialize)]
struct LogsQuery {
    #[serde(rename = "type")]
    log_type: String,
}

async fn stream_node_logs(
    State(db_pool): State<DbPool>,
    State(log_manager): State<Arc<LogManager>>,
    AuthUser(_user): AuthUser,
    Path(id): Path<String>,
    Query(query): Query<LogsQuery>,
) -> HttpResult<
    Sse<impl futures_util::Stream<Item = std::result::Result<Event, std::convert::Infallible>>>,
> {
    NodeRepo::get(&db_pool, &id).await?;

    let log_key = match query.log_type.as_str() {
        "setup" => LogKey::NodeSetup { node_id: id },
        "teardown" => LogKey::NodeTeardown { node_id: id },
        _ => {
            return Err(HttpError::bad_request(
                "log type must be 'setup' or 'teardown'",
            ));
        }
    };

    let log = log_manager
        .get_logger(&log_key)
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
