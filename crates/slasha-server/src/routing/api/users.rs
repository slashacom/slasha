use axum::{
    Json, Router,
    extract::{Path, State},
    response::IntoResponse,
    routing::{delete, get, patch, post},
};
use chrono::Utc;
use serde::Deserialize;
use slasha_db::{
    repos::{app::AppRepo, user::UserRepo},
    user::{User, UserRole},
};
use uuid::Uuid;

use crate::{
    HttpError, HttpResult,
    auth::hash_password,
    state::{AppState, Storage},
};

pub fn router() -> Router<AppState> {
    Router::new()
        .route("/", get(list_users))
        .route("/", post(create_user))
        .route("/{id}", get(get_user))
        .route("/{id}", patch(update_user))
        .route("/{id}", delete(delete_user))
}

async fn get_user(
    State(storage): State<Storage>,
    Path(id): Path<String>,
) -> HttpResult<impl IntoResponse> {
    let user = UserRepo::find_by_id(&storage.db_pool, &id).await?;
    let memberships = AppRepo::list_memberships_for_user(&storage.db_pool, &id).await?;
    let app_ids: Vec<String> = memberships.into_iter().map(|m| m.app_id).collect();

    Ok(Json(serde_json::json!({
        "user": user,
        "app_ids": app_ids,
    })))
}

async fn list_users(State(storage): State<Storage>) -> HttpResult<impl IntoResponse> {
    let all_users = UserRepo::list(&storage.db_pool).await?;

    Ok(Json(serde_json::json!({
        "users": all_users,
    })))
}

#[derive(Deserialize)]
struct CreateUserReq {
    email: String,
    password: String,
    role: UserRole,
    app_ids: Option<Vec<String>>,
}

async fn create_user(
    State(storage): State<Storage>,
    Json(payload): Json<CreateUserReq>,
) -> HttpResult<impl IntoResponse> {
    let hashed = hash_password(&payload.password)?;
    let new_user = User {
        id: Uuid::new_v4().to_string(),
        email: payload.email,
        password_hash: hashed,
        role: payload.role,
        created_at: Utc::now().naive_utc(),
        updated_at: Utc::now().naive_utc(),
    };

    let new_user = UserRepo::create(&storage.db_pool, new_user).await?;

    if let Some(app_ids) = payload.app_ids {
        AppRepo::set_user_memberships(&storage.db_pool, &new_user.id, app_ids).await?;
    }

    Ok(Json(serde_json::json!({
        "user": new_user,
    })))
}

#[derive(Deserialize)]
struct UpdateUserReq {
    email: Option<String>,
    role: Option<UserRole>,
    password: Option<String>,
    app_ids: Option<Vec<String>>,
}

async fn update_user(
    State(storage): State<Storage>,
    Path(id): Path<String>,
    Json(payload): Json<UpdateUserReq>,
) -> HttpResult<impl IntoResponse> {
    let user = UserRepo::find_by_id(&storage.db_pool, &id).await?;

    if let Some(new_role) = payload.role
        && user.role == UserRole::Admin
        && new_role == UserRole::User
    {
        let admin_count = UserRepo::admin_count(&storage.db_pool).await?;
        if admin_count == 1 {
            return Err(HttpError::bad_request(
                "There needs to be at least one admin user!",
            ));
        }
    }

    let password_hash = payload.password.map(|p| hash_password(&p)).transpose()?;

    let updated_user = UserRepo::update(
        &storage.db_pool,
        &id,
        payload.email,
        payload.role,
        password_hash,
    )
    .await?;

    if let Some(app_ids) = payload.app_ids {
        AppRepo::set_user_memberships(&storage.db_pool, &id, app_ids).await?;
    }

    Ok(Json(serde_json::json!({
        "user": updated_user,
    })))
}

async fn delete_user(
    State(storage): State<Storage>,
    Path(id): Path<String>,
) -> HttpResult<impl IntoResponse> {
    let user = UserRepo::find_by_id(&storage.db_pool, &id).await?;

    let admin_count = UserRepo::admin_count(&storage.db_pool).await?;

    if user.role == UserRole::Admin && admin_count == 1 {
        return Err(HttpError::bad_request(
            "There needs to be at least one admin user!",
        ));
    }

    UserRepo::delete(&storage.db_pool, &id).await?;

    Ok(Json(serde_json::json!({
        "deleted": true,
    })))
}
