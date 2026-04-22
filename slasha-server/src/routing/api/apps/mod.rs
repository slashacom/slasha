use crate::AppState;
use axum::Router;

pub mod deployments;
pub mod env;
pub mod files;
pub mod management;
pub mod service_env;
pub mod services;
mod utils;

pub fn router() -> Router<AppState> {
    Router::new()
        .merge(management::router())
        .nest("/{slug}/env", env::router())
        .nest("/{slug}/files", files::router())
        .nest("/{slug}/deployments", deployments::router())
        .nest("/{slug}/services", services::router())
        .nest("/{slug}/services/{service_id}/env", service_env::router())
}
