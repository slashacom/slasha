pub mod api;
pub mod app_access;
pub mod git;

use axum::Router;
#[cfg(feature = "embed-web")]
use axum::routing::get;
use tower_http::trace::TraceLayer;

use crate::AppState;
#[cfg(feature = "embed-web")]
use crate::assets::static_handler;

pub fn router(state: AppState) -> Router<AppState> {
    let router = Router::new()
        .nest("/_slasha/app-access", app_access::router())
        .nest("/api", api::router(state))
        .nest("/git", git::router())
        .layer(TraceLayer::new_for_http());

    #[cfg(feature = "embed-web")]
    let router = router
        .route("/", get(static_handler))
        .route("/{*path}", get(static_handler));

    router
}
