use axum::{Json, Router, routing::get};
use slasha_db::service::ServiceKind;
use strum::IntoEnumIterator;

use crate::{AppState, error::HttpResult};

pub fn router() -> Router<AppState> {
    Router::new().route("/kinds", get(get_service_kinds))
}

async fn get_service_kinds() -> HttpResult<Json<serde_json::Value>> {
    let kinds: Vec<serde_json::Value> = ServiceKind::iter()
        .map(|kind| {
            serde_json::json!({
                "name": kind.to_string(),
                "supported_versions": kind.supported_versions(),
                "default_env_vars": kind.generate_initial_env_vars(),
                "default_resources": {
                    "memory_bytes": kind.default_memory_bytes(),
                    "nano_cpus": kind.default_nano_cpus(),
                    "pids_limit": kind.default_pids_limit(),
                    "shm_size": kind.default_shm_size(),
                },
            })
        })
        .collect();

    Ok(Json(serde_json::json!({
        "kinds": kinds,
    })))
}
