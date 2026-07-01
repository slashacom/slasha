use axum::Router;

pub mod git;
pub mod github;

use crate::AppState;

pub fn router(state: AppState) -> Router<AppState> {
    Router::new()
        .merge(github::router(state.clone()))
        .nest("/git", git::router())
}
