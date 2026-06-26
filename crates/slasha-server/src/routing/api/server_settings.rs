use axum::{Json, Router, extract::State, routing::get};
use slasha_db::{
    DbPool, models::server_settings::ServerSettings, repos::server_settings::ServerSettingsRepo,
};

use crate::{AppState, error::HttpResult};

pub fn router() -> Router<AppState> {
    Router::new().route("/", get(get_settings).put(update_settings))
}

async fn get_settings(State(pool): State<DbPool>) -> HttpResult<Json<ServerSettings>> {
    let settings = ServerSettingsRepo::get(&pool).await?;
    Ok(Json(settings))
}

async fn update_settings(
    State(pool): State<DbPool>,
    Json(payload): Json<ServerSettings>,
) -> HttpResult<Json<ServerSettings>> {
    let updated = ServerSettingsRepo::update(&pool, payload).await?;
    Ok(Json(updated))
}
