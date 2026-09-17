use axum::{
    Json, Router,
    extract::{Path, State},
    middleware::from_fn_with_state,
    routing::{get, patch, post},
};
use chrono::Utc;
use garde::Validate;
use serde::Deserialize;
use serde_json::{Value, json};
use slasha_db::{
    models::s3_storage::{NewS3Storage, S3Storage, S3StorageChangeset},
    repos::s3_storage::S3StorageRepo,
};

use crate::{
    HttpError, HttpResult,
    extractors::ValidatedJson,
    middleware::admin::admin_middleware,
    routing::api::validation::not_empty,
    s3,
    state::{AppState, Storage},
};

pub fn router(state: AppState) -> Router<AppState> {
    let admin_routes = Router::new()
        .route("/", post(create_storage))
        .route("/{id}", patch(update_storage).delete(delete_storage))
        .route("/{id}/test", post(test_connection))
        .route_layer(from_fn_with_state(state, admin_middleware));

    Router::new()
        .route("/", get(list_storages))
        .route("/{id}", get(get_storage))
        .merge(admin_routes)
}

async fn list_storages(State(storage): State<Storage>) -> HttpResult<Json<Value>> {
    let storages = S3StorageRepo::list(&storage.db_pool).await?;
    Ok(Json(json!({ "storages": storages })))
}

#[derive(Deserialize, Validate)]
pub struct CreateS3StorageReq {
    #[garde(custom(not_empty))]
    pub name: String,
    #[garde(custom(not_empty))]
    pub endpoint: String,
    #[garde(custom(not_empty))]
    pub bucket: String,
    #[garde(skip)]
    #[serde(default = "default_region")]
    pub region: String,
    #[garde(custom(not_empty))]
    pub access_key_id: String,
    #[garde(custom(not_empty))]
    pub secret_access_key: String,
    #[garde(skip)]
    #[serde(default)]
    pub force_path_style: bool,
}

fn default_region() -> String {
    "auto".to_string()
}

async fn create_storage(
    State(storage): State<Storage>,
    ValidatedJson(payload): ValidatedJson<CreateS3StorageReq>,
) -> HttpResult<Json<Value>> {
    if S3StorageRepo::name_exists(&storage.db_pool, &payload.name, None).await? {
        return Err(HttpError::bad_request(format!(
            "S3 storage with name '{}' already exists",
            payload.name
        )));
    }

    let temp_storage = S3Storage {
        id: "test".to_string(),
        name: payload.name.clone(),
        endpoint: payload.endpoint.clone(),
        bucket: payload.bucket.clone(),
        region: payload.region.clone(),
        access_key_id: payload.access_key_id.clone(),
        secret_access_key: payload.secret_access_key.clone(),
        force_path_style: payload.force_path_style,
        created_at: Utc::now().naive_utc(),
        updated_at: Utc::now().naive_utc(),
    };

    s3::test_connection(&temp_storage)
        .await
        .map_err(|e| HttpError::bad_request(format!("S3 connection failed: {}", e)))?;

    let new_storage = NewS3Storage {
        name: payload.name,
        endpoint: payload.endpoint,
        bucket: payload.bucket,
        region: payload.region,
        access_key_id: payload.access_key_id,
        secret_access_key: payload.secret_access_key,
        force_path_style: payload.force_path_style,
    };

    let created = S3StorageRepo::create(&storage.db_pool, new_storage).await?;
    Ok(Json(json!({ "storage": created })))
}

async fn get_storage(
    State(storage): State<Storage>,
    Path(id): Path<String>,
) -> HttpResult<Json<Value>> {
    let item = S3StorageRepo::find(&storage.db_pool, &id).await?;
    Ok(Json(json!({ "storage": item })))
}

#[derive(Deserialize, Validate)]
pub struct UpdateS3StorageReq {
    #[garde(skip)]
    pub name: Option<String>,
    #[garde(skip)]
    pub endpoint: Option<String>,
    #[garde(skip)]
    pub bucket: Option<String>,
    #[garde(skip)]
    pub region: Option<String>,
    #[garde(skip)]
    pub access_key_id: Option<String>,
    #[garde(skip)]
    pub secret_access_key: Option<String>,
    #[garde(skip)]
    pub force_path_style: Option<bool>,
}

async fn update_storage(
    State(storage): State<Storage>,
    Path(id): Path<String>,
    ValidatedJson(payload): ValidatedJson<UpdateS3StorageReq>,
) -> HttpResult<Json<Value>> {
    let existing = S3StorageRepo::find(&storage.db_pool, &id).await?;

    if let Some(ref new_name) = payload.name
        && S3StorageRepo::name_exists(&storage.db_pool, new_name, Some(&id)).await?
    {
        return Err(HttpError::bad_request(format!(
            "S3 storage with name '{}' already exists",
            new_name
        )));
    }

    let test_storage = S3Storage {
        id: existing.id.clone(),
        name: payload.name.clone().unwrap_or(existing.name),
        endpoint: payload.endpoint.clone().unwrap_or(existing.endpoint),
        bucket: payload.bucket.clone().unwrap_or(existing.bucket),
        region: payload.region.clone().unwrap_or(existing.region),
        access_key_id: payload
            .access_key_id
            .clone()
            .unwrap_or(existing.access_key_id),
        secret_access_key: payload
            .secret_access_key
            .clone()
            .unwrap_or(existing.secret_access_key),
        force_path_style: payload
            .force_path_style
            .unwrap_or(existing.force_path_style),
        created_at: existing.created_at,
        updated_at: Utc::now().naive_utc(),
    };

    s3::test_connection(&test_storage)
        .await
        .map_err(|e| HttpError::bad_request(format!("S3 connection failed: {}", e)))?;

    let changeset = S3StorageChangeset {
        name: payload.name,
        endpoint: payload.endpoint,
        bucket: payload.bucket,
        region: payload.region,
        access_key_id: payload.access_key_id,
        secret_access_key: payload.secret_access_key,
        force_path_style: payload.force_path_style,
    };

    let updated = S3StorageRepo::update(&storage.db_pool, &id, changeset).await?;
    Ok(Json(json!({ "storage": updated })))
}

async fn delete_storage(
    State(storage): State<Storage>,
    Path(id): Path<String>,
) -> HttpResult<Json<Value>> {
    S3StorageRepo::find(&storage.db_pool, &id).await?;
    if S3StorageRepo::is_in_use(&storage.db_pool, &id).await? {
        return Err(HttpError::bad_request(
            "Cannot delete S3 storage: it is currently referenced by backup configurations or existing backups",
        ));
    }
    S3StorageRepo::delete(&storage.db_pool, &id).await?;
    Ok(Json(json!({ "deleted": true })))
}

async fn test_connection(
    State(storage): State<Storage>,
    Path(id): Path<String>,
) -> HttpResult<Json<Value>> {
    let item = S3StorageRepo::find(&storage.db_pool, &id).await?;

    s3::test_connection(&item)
        .await
        .map_err(|e| HttpError::bad_request(format!("S3 connection failed: {}", e)))?;

    Ok(Json(json!({ "ok": true })))
}
