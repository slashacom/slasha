use axum::{
    Json, Router,
    extract::{Path, Query, State},
    response::IntoResponse,
    routing::{delete, get, post, put},
};
use garde::Validate;
use serde::{Deserialize, Serialize};
use slasha_db::{
    DbPool, DuckdbPool,
    models::{
        logs::ResourceKind,
        node::{NewNode, Node, NodeChangeset, NodeStatus},
    },
    repos::{logs::LogsRepo, node::NodeRepo},
};
use uuid::Uuid;

use crate::{
    HttpError, HttpResult,
    extractors::{ValidatedJson, auth::AuthUser},
    logs::LogBus,
    routing::api::{
        logs::{LogQuery, fetch_resource_logs, stream_resource_logs},
        validation::not_empty,
    },
    state::AppState,
};

pub fn router() -> Router<AppState> {
    Router::new()
        .route("/", get(list_nodes))
        .route("/", post(create_node))
        .route("/{id}", get(get_node))
        .route("/{id}", put(update_node))
        .route("/{id}", delete(delete_node))
        .route("/{id}/logs", get(get_node_logs))
        .route("/{id}/stream", get(stream_node_logs))
}

#[derive(Serialize)]
pub struct NodeWithInfo {
    #[serde(flatten)]
    pub node: Node,
    pub connection_status: String,
    pub os: Option<String>,
}

async fn list_nodes(
    State(state): State<AppState>,
    AuthUser(_user): AuthUser,
) -> HttpResult<impl IntoResponse> {
    let nodes = NodeRepo::list(&state.storage.db_pool).await?;
    let mut results = Vec::new();

    for node in nodes {
        let (connection_status, os) = state.node_registry.resolve_node_info(&node).await;
        results.push(NodeWithInfo {
            node,
            connection_status,
            os,
        });
    }

    Ok(Json(serde_json::json!({ "nodes": results })))
}

async fn get_node(
    State(state): State<AppState>,
    AuthUser(_user): AuthUser,
    Path(id): Path<String>,
) -> HttpResult<impl IntoResponse> {
    let node = NodeRepo::get(&state.storage.db_pool, &id).await?;
    let (connection_status, os) = state.node_registry.resolve_node_info(&node).await;

    Ok(Json(serde_json::json!({
        "node": NodeWithInfo { node, connection_status, os }
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

    if let Err(e) = state
        .node_registry
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

    tokio::spawn({
        let db_pool = state.storage.db_pool.clone();
        let node_registry = state.node_registry.clone();
        let node = node.clone();
        let log_writer = state.runtime.log_bus.writer(ResourceKind::Node, &node.id);

        async move {
            let result = node_registry
                .run_ssh_script_streaming(&node, &setup_script, &log_writer)
                .await;

            let mut internal_root_ca = None;
            let status = match result {
                Ok(stdout_str) => {
                    if let Some(start) = stdout_str.find("---BEGIN ROOT CA---\n")
                        && let Some(end) = stdout_str[start..].find("\n---END ROOT CA---")
                    {
                        let cert_start = start + "---BEGIN ROOT CA---\n".len();
                        let cert_end = start + end;
                        if cert_start <= cert_end {
                            let cert = stdout_str[cert_start..cert_end].trim().to_string();
                            if !cert.is_empty() {
                                internal_root_ca = Some(cert);
                            }
                        }
                    }
                    tracing::info!(node_id = %node.id, node_name = %node.name, "node setup completed");
                    log_writer.stdout("setup completed successfully");
                    NodeStatus::Ready
                }
                Err(e) => {
                    tracing::error!(node_id = %node.id, node_name = %node.name, error = %e, "node setup failed");
                    log_writer.stdout(format!("setup failed: {}", e));
                    NodeStatus::Error
                }
            };

            if let Err(e) =
                NodeRepo::set_status_and_ca(&db_pool, &node.id, status, internal_root_ca).await
            {
                tracing::error!(node_id = %node.id, node_name = %node.name, error = %e, "failed to update node status and ca");
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

    if connection_changed {
        if node.is_local() {
            return Err(HttpError::bad_request(
                "can only update 'name' for the local node",
            ));
        }

        if NodeRepo::has_apps(&state.storage.db_pool, &id).await? {
            return Err(HttpError::bad_request(
                "cannot update node while it has apps assigned",
            ));
        }
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
            .node_registry
            .probe_ssh(&node)
            .await
            .map_err(|e| HttpError::bad_request(e.to_string()))?;
    }

    let node = NodeRepo::update(&state.storage.db_pool, &id, changeset).await?;
    state.node_registry.remove(&node);

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

    let log_writer = state.runtime.log_bus.writer(ResourceKind::Node, &node.id);

    tokio::spawn({
        let db_pool = state.storage.db_pool;
        let duckdb_pool = state.storage.duckdb_pool;
        let node_registry = state.node_registry;
        let log_bus = state.runtime.log_bus;

        async move {
            match node_registry
                .run_ssh_script_streaming(&node, TEARDOWN_SCRIPT, &log_writer)
                .await
            {
                Ok(_) => {
                    tracing::info!(node_id = %node.id, node_name = %node.name, "node teardown completed");
                    log_writer.stdout("node teardown completed successfully");

                    if let Err(e) = NodeRepo::delete(&db_pool, &node.id).await {
                        tracing::error!(node_id = %node.id, error = %e, "failed to delete node from database");
                    }
                    log_bus.remove(&node.id);
                    let _ = LogsRepo::delete_by_resource_id(&duckdb_pool, &node.id).await;
                }
                Err(e) => {
                    tracing::error!(node_id = %node.id, error = %e, "node teardown failed");
                    log_writer.stdout(format!("node teardown failed: {}", e));

                    if let Err(db_err) =
                        NodeRepo::set_status(&db_pool, &node.id, NodeStatus::Error).await
                    {
                        tracing::error!(node_id = %node.id, error = %db_err, "failed to set node status to error");
                    }
                }
            }

            node_registry.remove(&node);
        }
    });

    Ok(axum::http::StatusCode::NO_CONTENT)
}

async fn get_node_logs(
    State(db_pool): State<DbPool>,
    State(duckdb_pool): State<DuckdbPool>,
    AuthUser(_user): AuthUser,
    Path(id): Path<String>,
    Query(query): Query<LogQuery>,
) -> HttpResult<impl IntoResponse> {
    NodeRepo::get(&db_pool, &id).await?;
    fetch_resource_logs(&duckdb_pool, &id, query).await
}

async fn stream_node_logs(
    State(db_pool): State<DbPool>,
    State(log_bus): State<LogBus>,
    AuthUser(_user): AuthUser,
    Path(id): Path<String>,
) -> HttpResult<impl IntoResponse> {
    NodeRepo::get(&db_pool, &id).await?;
    stream_resource_logs(&log_bus, &id).await
}
