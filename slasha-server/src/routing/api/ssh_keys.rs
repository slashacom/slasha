use axum::{
    Json, Router,
    extract::{Path, State},
    routing::{delete, get, post},
};
use serde::{Deserialize, Serialize};
use serde_json::{Value, json};
use slasha_db::{repos::ssh_key::SshKeyRepo, ssh_keys::SshKey};
use uuid::Uuid;

use crate::{
    error::Result,
    extractors::auth::AuthUser,
    ssh::regenerate_authorized_keys,
    state::{AppState, Storage},
};

pub fn router() -> Router<AppState> {
    Router::new()
        .route("/", get(list_ssh_keys))
        .route("/", post(create_ssh_key))
        .route("/{id}", delete(delete_ssh_key))
}

#[derive(Serialize)]
pub struct ListSshKeysResponse {
    pub keys: Vec<SshKey>,
}

async fn list_ssh_keys(
    State(storage): State<Storage>,
    AuthUser(user): AuthUser,
) -> Result<Json<ListSshKeysResponse>> {
    let keys = SshKeyRepo::list_for_user(&storage.db_pool, &user.id).await?;

    Ok(Json(ListSshKeysResponse { keys }))
}

#[derive(Deserialize)]
pub struct CreateSshKeyRequest {
    pub title: Option<String>,
    pub public_key: String,
}

async fn create_ssh_key(
    State(storage): State<Storage>,
    AuthUser(user): AuthUser,
    Json(payload): Json<CreateSshKeyRequest>,
) -> Result<Json<SshKey>> {
    let now = chrono::Utc::now().naive_utc();
    let new_key = SshKey {
        id: Uuid::new_v4().to_string(),
        user_id: user.id.clone(),
        title: payload.title,
        public_key: payload.public_key,
        created_at: now,
    };

    let new_key = SshKeyRepo::create(&storage.db_pool, new_key).await?;

    regenerate_authorized_keys(&storage).await?;

    Ok(Json(new_key))
}

async fn delete_ssh_key(
    State(storage): State<Storage>,
    AuthUser(user): AuthUser,
    Path(id): Path<String>,
) -> Result<Json<Value>> {
    SshKeyRepo::delete(&storage.db_pool, &id, &user.id).await?;

    regenerate_authorized_keys(&storage).await?;

    Ok(Json(json!({ "status": "ok" })))
}
