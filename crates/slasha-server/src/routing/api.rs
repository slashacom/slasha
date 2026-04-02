use crate::error::Result;
use axum::{Json, Router, routing::get};
use serde_json::{Value, json};

pub fn router() -> Router {
    Router::new().route("/health", get(health_check))
}

async fn health_check() -> Result<Json<Value>> {
    Ok(Json(
        json!({ "status": "ok", "version": env!("CARGO_PKG_VERSION") }),
    ))
}
