use crate::{
    error::{Error, Result},
    extractors::auth::AuthUser,
    ssh::regenerate_authorized_keys,
    state::{AppState, Storage},
};
use axum::{
    Json, Router,
    extract::{Path, State},
    routing::{delete, get, post},
};
use diesel::prelude::*;
use models::{schema::ssh_keys, ssh_keys::SshKey};
use serde::{Deserialize, Serialize};
use serde_json::{Value, json};
use uuid::Uuid;

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
    let mut conn = storage.db_pool.get()?;

    let keys = ssh_keys::table
        .filter(ssh_keys::user_id.eq(&user.id))
        .load::<SshKey>(&mut conn)?;

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
    let mut conn = storage.db_pool.get()?;

    let now = chrono::Utc::now().naive_utc();
    let new_key = SshKey {
        id: Uuid::new_v4().to_string(),
        user_id: user.id.clone(),
        title: payload.title,
        public_key: payload.public_key,
        created_at: now,
    };

    diesel::insert_into(ssh_keys::table)
        .values(&new_key)
        .execute(&mut conn)?;

    regenerate_authorized_keys(&storage)?;

    Ok(Json(new_key))
}

async fn delete_ssh_key(
    State(storage): State<Storage>,
    AuthUser(user): AuthUser,
    Path(id): Path<String>,
) -> Result<Json<Value>> {
    let mut conn = storage.db_pool.get()?;

    let deleted_rows = diesel::delete(
        ssh_keys::table
            .filter(ssh_keys::id.eq(&id))
            .filter(ssh_keys::user_id.eq(&user.id)),
    )
    .execute(&mut conn)?;

    if deleted_rows == 0 {
        return Err(Error::NotFound("SSH key not found".into()));
    }

    regenerate_authorized_keys(&storage)?;

    Ok(Json(json!({ "status": "ok" })))
}
