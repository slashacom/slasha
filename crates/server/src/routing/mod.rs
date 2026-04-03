pub mod api;

#[cfg(feature = "bundle")]
use crate::assets::static_handler;
#[cfg(feature = "bundle")]
use axum::routing::get;

use crate::AppState;
use axum::Router;
use tower_http::trace::TraceLayer;

pub fn router() -> Router<AppState> {
    let router = Router::new()
        .nest("/api", api::router())
        .layer(TraceLayer::new_for_http());

    #[cfg(feature = "bundle")]
    let router = router
        .route("/", get(static_handler))
        .route("/{*path}", get(static_handler));

    router
}
