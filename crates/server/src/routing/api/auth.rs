use axum::{
    Json, Router,
    extract::State,
    response::IntoResponse,
    routing::{get, post},
};
use axum_extra::{
    TypedHeader,
    headers::{Authorization, authorization::Bearer},
};
use chrono::Utc;
use diesel::prelude::*;
use jsonwebtoken::{DecodingKey, Validation, decode};
use serde::Deserialize;
use uuid::Uuid;

use crate::{
    AppState,
    auth::{TokenPayload, create_jwt, hash_password, verify_password},
    error::{Error, Result},
    models::user::User,
    schema::users,
};

const EXP_TIME: usize = 30 * 24 * 60 * 60;

pub fn router() -> Router<AppState> {
    Router::new()
        .route("/signup", post(signup))
        .route("/login", post(login))
        .route("/me", get(me))
        .route("/status", get(status))
}

async fn status(State(state): State<AppState>) -> Result<impl IntoResponse> {
    let mut conn = state
        .db_pool
        .get()
        .map_err(|e| Error::Internal(anyhow::anyhow!("DB pool error: {}", e)))?;

    let admin_count: i64 = users::table
        .filter(users::role.eq("admin"))
        .count()
        .get_result(&mut conn)
        .map_err(|e| Error::Internal(anyhow::anyhow!("Database error: {}", e)))?;

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
    State(state): State<AppState>,
    Json(payload): Json<SignupReq>,
) -> Result<impl IntoResponse> {
    let mut conn = state
        .db_pool
        .get()
        .map_err(|e| Error::Internal(anyhow::anyhow!("DB pool error: {}", e)))?;

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

    let token = create_jwt(&token_payload, &state.jwt_secret)?;

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
    State(state): State<AppState>,
    Json(payload): Json<LoginReq>,
) -> Result<impl IntoResponse> {
    let mut conn = state
        .db_pool
        .get()
        .map_err(|e| Error::Internal(anyhow::anyhow!("DB pool error: {}", e)))?;

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

    let token = create_jwt(&token_payload, &state.jwt_secret)?;

    Ok(Json(serde_json::json!({
        "token": token,
        "user": user,
    })))
}

async fn me(
    State(state): State<AppState>,
    auth: Option<TypedHeader<Authorization<Bearer>>>,
) -> Result<impl IntoResponse> {
    let auth = auth.ok_or(Error::Unauthorized)?;
    let token = auth.token();

    let decoding_key = DecodingKey::from_secret(state.jwt_secret.as_bytes());
    let mut validation = Validation::default();
    validation.validate_exp = true;

    let token_data = decode::<TokenPayload>(token, &decoding_key, &validation)
        .map_err(|_| Error::Unauthorized)?;

    let mut conn = state
        .db_pool
        .get()
        .map_err(|e| Error::Internal(anyhow::anyhow!("DB pool error: {}", e)))?;

    let user = users::table
        .filter(users::id.eq(&token_data.claims.id))
        .first::<User>(&mut conn)
        .optional()?;

    let user = match user {
        Some(u) => u,
        None => return Err(Error::Unauthorized),
    };

    Ok(Json(serde_json::json!({
        "user": user
    })))
}
