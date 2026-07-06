use axum::{
    Json, Router,
    extract::{Path, State},
    routing::{delete, get, post},
};
use garde::Validate;
use crate::routing::api::validation::not_empty;
use serde::{Deserialize, Serialize};
use serde_json::{Value, json};
use slasha_db::{
    repos::ssh_key::SshKeyRepo,
    ssh_keys::{NewSshKey, SshKey},
};

use crate::{
    HttpResult,
    extractors::{ValidatedJson, auth::AuthUser},
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
) -> HttpResult<Json<ListSshKeysResponse>> {
    let keys = SshKeyRepo::list_for_user(&storage.db_pool, &user.id).await?;

    Ok(Json(ListSshKeysResponse { keys }))
}

#[derive(Deserialize, Validate)]
pub struct CreateSshKeyRequest {
    #[garde(skip)]
    pub title: Option<String>,
    #[garde(custom(not_empty))]
    pub public_key: String,
}

async fn create_ssh_key(
    State(storage): State<Storage>,
    AuthUser(user): AuthUser,
    ValidatedJson(payload): ValidatedJson<CreateSshKeyRequest>,
) -> HttpResult<Json<SshKey>> {
    let new_key = NewSshKey {
        user_id: user.id.clone(),
        title: payload.title,
        public_key: payload.public_key,
    };

    let new_key = SshKeyRepo::create(&storage.db_pool, new_key).await?;

    regenerate_authorized_keys(&storage).await?;

    Ok(Json(new_key))
}

async fn delete_ssh_key(
    State(storage): State<Storage>,
    AuthUser(user): AuthUser,
    Path(id): Path<String>,
) -> HttpResult<Json<Value>> {
    SshKeyRepo::delete(&storage.db_pool, &id, &user.id).await?;

    regenerate_authorized_keys(&storage).await?;

    Ok(Json(json!({ "status": "ok" })))
}
