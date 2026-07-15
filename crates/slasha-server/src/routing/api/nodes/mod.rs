use axum::{Router, routing::get};

use crate::state::AppState;

pub mod management;
pub mod metrics;

pub fn router() -> Router<AppState> {
    Router::new()
        .merge(management::router())
        .route("/{id}/metrics", get(metrics::get_node_metrics))
}
