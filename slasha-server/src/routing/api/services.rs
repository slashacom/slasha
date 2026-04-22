use axum::{Json, Router, routing::get};
use models::service::ServiceKind;
use strum::IntoEnumIterator;

use crate::{AppState, error::Result};

pub fn router() -> Router<AppState> {
    Router::new().route("/kinds", get(get_service_kinds))
}

async fn get_service_kinds() -> Result<Json<serde_json::Value>> {
    let kinds: Vec<serde_json::Value> = ServiceKind::iter()
        .map(|kind| {
            serde_json::json!({
                "name": kind.to_string(),
                "supported_versions": kind.supported_versions(),
            })
        })
        .collect();

    Ok(Json(serde_json::json!({
        "kinds": kinds,
    })))
}
