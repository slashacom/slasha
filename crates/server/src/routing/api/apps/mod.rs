use crate::AppState;
use axum::Router;

pub mod files;
pub mod management;
mod utils;

pub fn router() -> Router<AppState> {
    Router::new()
        .merge(management::router())
        .nest("/{slug}/files", files::router())
}
