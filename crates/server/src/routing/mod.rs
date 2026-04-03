pub mod api;

use crate::assets::static_handler;
use axum::Router;
use axum::routing::get;
use tower_http::trace::TraceLayer;

pub fn router() -> Router {
    Router::new()
        .nest("/api", api::router())
        .route("/", get(static_handler))
        .route("/{*path}", get(static_handler))
        .layer(TraceLayer::new_for_http())
}
