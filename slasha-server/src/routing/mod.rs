pub mod api;
pub mod git;

use axum::Router;
#[cfg(feature = "bundle")]
use axum::routing::get;
use tower_http::trace::TraceLayer;

use crate::AppState;
#[cfg(feature = "bundle")]
use crate::assets::static_handler;

pub fn router(state: AppState) -> Router<AppState> {
    let router = Router::new()
        .nest("/api", api::router(state))
        .nest("/git", git::router())
        .layer(TraceLayer::new_for_http());

    #[cfg(feature = "bundle")]
    let router = router
        .route("/", get(static_handler))
        .route("/{*path}", get(static_handler));

    router
}
