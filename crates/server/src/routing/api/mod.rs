use crate::{AppState, error::Result};
use axum::{Json, Router, routing::get};
use serde_json::{Value, json};

pub mod apps;
pub mod auth;

pub fn router() -> Router<AppState> {
    Router::new()
        .route("/health", get(health_check))
        .nest("/auth", auth::router())
        .nest("/apps", apps::router())
}

async fn health_check() -> Result<Json<Value>> {
    Ok(Json(
        json!({ "status": "ok", "version": env!("CARGO_PKG_VERSION") }),
    ))
}
