use axum::{Json, Router, routing::get};
use serde_json::{Value, json};

use crate::{AppState, error::HttpResult};

pub mod alert_rules;
pub mod apps;
pub mod auth;
pub mod channels;
pub mod monitoring;
pub mod service_kinds;
pub mod ssh_keys;
pub mod users;

use crate::middleware::admin::admin_middleware;

pub fn router(state: AppState) -> Router<AppState> {
    Router::new()
        .route("/health", get(health_check))
        .nest("/auth", auth::router())
        .nest("/apps", apps::router())
        .nest("/monitoring", monitoring::router())
        .nest("/services", service_kinds::router())
        .nest("/ssh-keys", ssh_keys::router())
        .nest(
            "/channels",
            channels::router().route_layer(axum::middleware::from_fn_with_state(
                state.clone(),
                admin_middleware,
            )),
        )
        .nest(
            "/alert-rules",
            alert_rules::router().route_layer(axum::middleware::from_fn_with_state(
                state.clone(),
                admin_middleware,
            )),
        )
        .nest(
            "/users",
            users::router().route_layer(axum::middleware::from_fn_with_state(
                state,
                admin_middleware,
            )),
        )
}

async fn health_check() -> HttpResult<Json<Value>> {
    Ok(Json(
        json!({ "status": "ok", "version": env!("CARGO_PKG_VERSION") }),
    ))
}
