use axum::{
    Json, Router,
    extract::{Path, State},
    routing::{get, post, put},
};
use serde::Deserialize;
use serde_json::{Value, json};
use slasha_db::{DbPool, models::channel::Channel, repos::channel::ChannelRepo};

use crate::{
    AppState,
    alerting::actions,
    error::{HttpError, HttpResult},
};

pub fn router() -> Router<AppState> {
    Router::new()
        .route("/", get(list_channels).post(create_channel))
        .route("/{id}", put(update_channel).delete(delete_channel))
        .route("/{id}/test", post(test_channel))
}

#[derive(Deserialize)]
struct CreateChannelRequest {
    name: String,
    kind: String,
    config: Value,
}

#[derive(Deserialize)]
struct UpdateChannelRequest {
    name: String,
    config: Value,
}

async fn list_channels(State(pool): State<DbPool>) -> HttpResult<Json<Vec<Channel>>> {
    let channels = ChannelRepo::list(&pool).await?;
    Ok(Json(channels))
}

async fn create_channel(
    State(pool): State<DbPool>,
    Json(payload): Json<CreateChannelRequest>,
) -> HttpResult<Json<Channel>> {
    let channel = ChannelRepo::create(
        &pool,
        payload.name,
        payload.kind,
        payload.config.to_string(),
    )
    .await?;
    Ok(Json(channel))
}

async fn update_channel(
    State(pool): State<DbPool>,
    Path(id): Path<String>,
    Json(payload): Json<UpdateChannelRequest>,
) -> HttpResult<Json<Channel>> {
    let channel = ChannelRepo::update(&pool, &id, payload.name, payload.config.to_string()).await?;
    Ok(Json(channel))
}

async fn delete_channel(
    State(pool): State<DbPool>,
    Path(id): Path<String>,
) -> HttpResult<Json<Value>> {
    ChannelRepo::delete(&pool, &id).await?;
    Ok(Json(json!({ "status": "ok" })))
}

async fn test_channel(
    State(pool): State<DbPool>,
    Path(id): Path<String>,
) -> HttpResult<Json<Value>> {
    let channel = ChannelRepo::get(&pool, &id).await?;
    actions::send_via_channel(&channel, "✅ slasha test alert — this channel is working.")
        .await
        .map_err(|err| HttpError::bad_request(err.to_string()))?;
    Ok(Json(json!({ "status": "ok" })))
}
