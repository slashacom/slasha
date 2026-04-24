use axum::{
    Json, Router,
    extract::{Path, State},
    response::IntoResponse,
    routing::{delete, get, patch, post},
};
use chrono::Utc;
use diesel::prelude::*;
use serde::Deserialize;
use uuid::Uuid;

use crate::{
    AppState,
    auth::hash_password,
    error::{Error, Result},
};

use models::{schema::users, user::User};

pub fn router() -> Router<AppState> {
    Router::new()
        .route("/", get(list_users))
        .route("/", post(create_user))
        .route("/{id}", get(get_user))
        .route("/{id}", patch(update_user))
        .route("/{id}", delete(delete_user))
}

async fn get_user(
    State(state): State<AppState>,
    Path(id): Path<String>,
) -> Result<impl IntoResponse> {
    let mut conn = state.db_pool.get()?;

    let user = users::table
        .filter(users::id.eq(&id))
        .first::<User>(&mut conn)
        .optional()?
        .ok_or_else(|| Error::NotFound("User not found".into()))?;

    Ok(Json(serde_json::json!({
        "user": user,
    })))
}

async fn list_users(State(state): State<AppState>) -> Result<impl IntoResponse> {
    let mut conn = state.db_pool.get()?;

    let all_users = users::table
        .order(users::created_at.desc())
        .load::<User>(&mut conn)?;

    Ok(Json(serde_json::json!({
        "users": all_users,
    })))
}

#[derive(Deserialize)]
struct CreateUserReq {
    email: String,
    password: String,
    role: String,
}

async fn create_user(
    State(state): State<AppState>,
    Json(payload): Json<CreateUserReq>,
) -> Result<impl IntoResponse> {
    let mut conn = state.db_pool.get()?;

    let hashed = hash_password(&payload.password)?;
    let new_user = User {
        id: Uuid::new_v4().to_string(),
        email: payload.email,
        password_hash: hashed,
        role: payload.role,
        created_at: Utc::now().naive_utc(),
        updated_at: Utc::now().naive_utc(),
    };

    diesel::insert_into(users::table)
        .values(&new_user)
        .execute(&mut conn)?;

    Ok(Json(serde_json::json!({
        "user": new_user,
    })))
}

#[derive(Deserialize)]
struct UpdateUserReq {
    email: Option<String>,
    role: Option<String>,
}

async fn update_user(
    State(state): State<AppState>,
    Path(id): Path<String>,
    Json(payload): Json<UpdateUserReq>,
) -> Result<impl IntoResponse> {
    let mut conn = state.db_pool.get()?;

    let updated_at = Utc::now().naive_utc();

    if let Some(email) = payload.email {
        diesel::update(users::table.filter(users::id.eq(&id)))
            .set((users::email.eq(email), users::updated_at.eq(updated_at)))
            .execute(&mut conn)?;
    }

    if let Some(role) = payload.role {
        diesel::update(users::table.filter(users::id.eq(&id)))
            .set((users::role.eq(role), users::updated_at.eq(updated_at)))
            .execute(&mut conn)?;
    }

    let updated_user = users::table
        .filter(users::id.eq(&id))
        .first::<User>(&mut conn)?;

    Ok(Json(serde_json::json!({
        "user": updated_user,
    })))
}

async fn delete_user(
    State(state): State<AppState>,
    Path(id): Path<String>,
) -> Result<impl IntoResponse> {
    let mut conn = state.db_pool.get()?;

    let user = users::table
        .filter(users::id.eq(&id))
        .first::<User>(&mut conn)
        .optional()?
        .ok_or_else(|| Error::NotFound("User not found".into()))?;

    let admin_count = users::table
        .filter(users::role.eq("admin"))
        .count()
        .get_result::<i64>(&mut conn)?;

    if user.role == "admin" && admin_count == 1 {
        return Err(Error::Internal(anyhow::anyhow!(
            "There needs to be at least one admin user!"
        )));
    }

    diesel::delete(users::table.filter(users::id.eq(&id))).execute(&mut conn)?;

    Ok(Json(serde_json::json!({
        "deleted": true,
    })))
}
