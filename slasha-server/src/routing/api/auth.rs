use axum::{
    Json, Router,
    extract::State,
    response::IntoResponse,
    routing::{get, post},
};
use chrono::Utc;
use diesel::prelude::*;
use serde::Deserialize;
use uuid::Uuid;

use crate::{
    auth::{TokenPayload, create_jwt, hash_password, verify_password},
    error::{Error, Result},
    extractors::auth::AuthUser,
    state::{AppState, Config, Storage},
};

use models::{schema::users, user::User};

const EXP_TIME: usize = 30 * 24 * 60 * 60;

pub fn router() -> Router<AppState> {
    Router::new()
        .route("/signup", post(signup))
        .route("/login", post(login))
        .route("/me", get(me))
        .route("/status", get(status))
}

async fn status(State(storage): State<Storage>) -> Result<impl IntoResponse> {
    let mut conn = storage.db_pool.get()?;

    let admin_count: i64 = users::table
        .filter(users::role.eq("admin"))
        .count()
        .get_result(&mut conn)?;

    Ok(Json(serde_json::json!({
        "has_admin": admin_count > 0,
    })))
}

#[derive(Deserialize)]
pub struct SignupReq {
    pub email: String,
    pub password: String,
}

async fn signup(
    State(storage): State<Storage>,
    State(config): State<Config>,
    Json(payload): Json<SignupReq>,
) -> Result<impl IntoResponse> {
    let mut conn = storage.db_pool.get()?;

    let admin_count: i64 = users::table
        .filter(users::role.eq("admin"))
        .count()
        .get_result(&mut conn)?;

    if admin_count > 0 {
        return Err(Error::BadRequest("An admin already exists".into()));
    }

    let hashed = hash_password(&payload.password)?;
    let new_user = User {
        id: Uuid::new_v4().to_string(),
        email: payload.email.clone(),
        password_hash: hashed,
        role: "admin".into(),
        created_at: Utc::now().naive_utc(),
        updated_at: Utc::now().naive_utc(),
    };

    diesel::insert_into(users::table)
        .values(&new_user)
        .execute(&mut conn)?;

    let exp = Utc::now().timestamp() as usize + EXP_TIME;
    let token_payload = TokenPayload {
        id: new_user.id.clone(),
        email: new_user.email.clone(),
        exp,
    };

    let token = create_jwt(&token_payload, &config.jwt_secret)?;

    Ok(Json(serde_json::json!({
        "token": token,
        "user": new_user,
    })))
}

#[derive(Deserialize)]
pub struct LoginReq {
    pub email: String,
    pub password: String,
}

async fn login(
    State(storage): State<Storage>,
    State(config): State<Config>,
    Json(payload): Json<LoginReq>,
) -> Result<impl IntoResponse> {
    let mut conn = storage.db_pool.get()?;

    let user = users::table
        .filter(users::email.eq(&payload.email))
        .first::<User>(&mut conn)
        .optional()?;

    let user = match user {
        Some(u) => u,
        None => return Err(Error::BadRequest("Invalid email or password".into())),
    };

    let is_valid = verify_password(&payload.password, &user.password_hash)?;
    if !is_valid {
        return Err(Error::BadRequest("Invalid email or password".into()));
    }

    let exp = Utc::now().timestamp() as usize + EXP_TIME;
    let token_payload = TokenPayload {
        id: user.id.clone(),
        email: user.email.clone(),
        exp,
    };

    let token = create_jwt(&token_payload, &config.jwt_secret)?;

    Ok(Json(serde_json::json!({
        "token": token,
        "user": user,
    })))
}

async fn me(AuthUser(user): AuthUser) -> Result<impl IntoResponse> {
    Ok(Json(serde_json::json!({
        "user": user
    })))
}
