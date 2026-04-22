use crate::{AppState, error::Result};
use axum::{Json, Router, routing::get};
use serde_json::{Value, json};

pub mod apps;
pub mod auth;
pub mod services;
pub mod ssh_keys;
pub mod users;

use crate::middleware::admin::admin_middleware;

pub fn router(state: AppState) -> Router<AppState> {
    Router::new()
        .route("/health", get(health_check))
        .nest("/auth", auth::router())
        .nest("/apps", apps::router())
        .nest("/services", services::router())
        .nest("/ssh-keys", ssh_keys::router())
        .nest(
            "/users",
            users::router().route_layer(axum::middleware::from_fn_with_state(
                state,
                admin_middleware,
            )),
        )
}

async fn health_check() -> Result<Json<Value>> {
    Ok(Json(
        json!({ "status": "ok", "version": env!("CARGO_PKG_VERSION") }),
    ))
}
