use crate::AppState;
use axum::Router;

pub mod deployments;
pub mod files;
pub mod management;
mod utils;

pub fn router() -> Router<AppState> {
    Router::new()
        .merge(management::router())
        .nest("/{slug}/files", files::router())
        .nest("/{slug}/deployments", deployments::router())
}
