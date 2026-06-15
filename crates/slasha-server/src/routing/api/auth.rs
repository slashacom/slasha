use std::time::Duration;

use axum::{
    Json, Router,
    extract::State,
    middleware,
    response::IntoResponse,
    routing::{get, post},
};
use chrono::Utc;
use serde::Deserialize;
use slasha_db::{repos::user::UserRepo, user::User};
use uuid::Uuid;

use crate::{
    auth::{TokenPayload, create_jwt, hash_password, verify_password},
    error::{HttpError, HttpResult},
    extractors::auth::AuthUser,
    middleware::rate_limit::{RateLimit, RateLimiter, rate_limit_middleware},
    state::{AppState, Config, Storage},
};

const EXP_TIME: usize = 30 * 24 * 60 * 60;

pub fn router() -> Router<AppState> {
    let login_limiter = RateLimiter::new(RateLimit {
        max_requests: 10,
        window: Duration::from_secs(60),
    });
    let signup_limiter = RateLimiter::new(RateLimit {
        max_requests: 3,
        window: Duration::from_secs(60),
    });

    let auth_routes = Router::new()
        .route(
            "/signup",
            post(signup).layer(middleware::from_fn_with_state(
                signup_limiter,
                rate_limit_middleware,
            )),
        )
        .route(
            "/login",
            post(login).layer(middleware::from_fn_with_state(
                login_limiter,
                rate_limit_middleware,
            )),
        );

    Router::new()
        .merge(auth_routes)
        .route("/me", get(me))
        .route("/status", get(status))
}

async fn status(State(storage): State<Storage>) -> HttpResult<impl IntoResponse> {
    let admin_count = UserRepo::admin_count(&storage.db_pool).await?;

    Ok(Json(serde_json::json!({
        "has_admin": admin_count > 0,
    })))
}

#[derive(Deserialize)]
pub struct SignupReq {
    pub email: String,
    pub password: String,
    pub confirm_password: String,
}

async fn signup(
    State(storage): State<Storage>,
    State(config): State<Config>,
    Json(payload): Json<SignupReq>,
) -> HttpResult<impl IntoResponse> {
    let admin_count = UserRepo::admin_count(&storage.db_pool).await?;

    if admin_count > 0 {
        return Err(HttpError::bad_request("An admin already exists"));
    }

    if payload.password != payload.confirm_password {
        return Err(HttpError::bad_request("Passwords do not match"));
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

    UserRepo::create(&storage.db_pool, new_user.clone()).await?;

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
) -> HttpResult<impl IntoResponse> {
    let user = UserRepo::find_by_email(&storage.db_pool, &payload.email).await?;

    let user = match user {
        Some(u) => u,
        None => return Err(HttpError::bad_request("Invalid email or password")),
    };

    let is_valid = verify_password(&payload.password, &user.password_hash)?;
    if !is_valid {
        return Err(HttpError::bad_request("Invalid email or password"));
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

async fn me(AuthUser(user): AuthUser) -> HttpResult<impl IntoResponse> {
    Ok(Json(serde_json::json!({
        "user": user
    })))
}
