use axum::{Router, middleware::from_fn_with_state};

use crate::{middleware::admin::admin_middleware, state::AppState};

pub mod console;
pub mod management;
pub mod metrics;

pub fn router(state: AppState) -> Router<AppState> {
    Router::new()
        .merge(management::router(state.clone()))
        .merge(metrics::router().route_layer(from_fn_with_state(state.clone(), admin_middleware)))
        .merge(console::router().route_layer(from_fn_with_state(state, admin_middleware)))
}
