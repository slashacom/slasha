use axum::{Json, Router, extract::State, response::IntoResponse, routing::get};

use crate::{AppState, HttpResult, docker::AppDocker, extractors::app::ActiveApp};

pub fn router() -> Router<AppState> {
    Router::new().route("/", get(list_volumes))
}

async fn list_volumes(
    State(state): State<AppState>,
    ActiveApp { app, .. }: ActiveApp,
) -> HttpResult<impl IntoResponse> {
    let app_docker = AppDocker::new(state, app).await?;
    let volumes = app_docker.list_volumes().await?;

    Ok(Json(serde_json::json!({ "volumes": volumes })))
}
