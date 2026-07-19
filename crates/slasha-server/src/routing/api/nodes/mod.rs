use axum::Router;

use crate::state::AppState;

pub mod management;
pub mod metrics;

pub fn router() -> Router<AppState> {
    Router::new()
        .merge(management::router())
        .merge(metrics::router())
}
