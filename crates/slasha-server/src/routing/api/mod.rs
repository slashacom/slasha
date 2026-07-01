use axum::{Json, Router, routing::get};
use serde_json::{Value, json};

use crate::{AppState, error::HttpResult};

pub mod alerts;
pub mod apps;
pub mod auth;
pub mod connections;
pub mod monitoring;
pub mod service_kinds;
pub mod ssh_keys;
pub mod users;

use crate::middleware::admin::admin_middleware;

pub fn router(state: AppState) -> Router<AppState> {
    Router::new()
        .route("/health", get(health_check))
        .nest("/auth", auth::router())
        .nest("/connections", connections::router(state.clone()))
        .nest("/apps", apps::router())
        .nest(
            "/alerts",
            alerts::router().route_layer(axum::middleware::from_fn_with_state(
                state.clone(),
                admin_middleware,
            )),
        )
        .nest("/monitoring", monitoring::router())
        .nest("/services", service_kinds::router())
        .nest("/ssh-keys", ssh_keys::router())
        .nest(
            "/users",
            users::router().route_layer(axum::middleware::from_fn_with_state(
                state,
                admin_middleware,
            )),
        )
}

async fn health_check(
    axum::extract::State(state): axum::extract::State<AppState>,
) -> HttpResult<Json<Value>> {
    let mut status = "ok";
    let mut db_status = "ok";
    let mut docker_status = "ok";

    if let Err(e) = state.storage.db_pool.get() {
        tracing::error!("DB health check failed: {}", e);
        db_status = "error";
        status = "error";
    }

    if let Err(e) = state.clients.docker.ping().await {
        tracing::error!("Docker health check failed: {}", e);
        docker_status = "error";
        status = "error";
    }

    Ok(Json(json!({
        "status": status,
        "version": env!("CARGO_PKG_VERSION"),
        "services": {
            "database": db_status,
            "docker": docker_status,
        }
    })))
}
