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
        .route("/update", post(update_profile))
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

#[derive(Deserialize)]
pub struct UpdateProfileReq {
    pub email: Option<String>,
    pub current_password: Option<String>,
    pub new_password: Option<String>,
    pub confirm_new_password: Option<String>,
}

async fn update_profile(
    AuthUser(user): AuthUser,
    State(storage): State<Storage>,
    State(config): State<Config>,
    Json(payload): Json<UpdateProfileReq>,
) -> HttpResult<impl IntoResponse> {
    if payload.email.is_some() || payload.new_password.is_some() {
        let current_pwd = payload.current_password.as_deref().ok_or_else(|| {
            HttpError::bad_request("Current password is required to update settings")
        })?;

        let is_valid = verify_password(current_pwd, &user.password_hash)?;
        if !is_valid {
            return Err(HttpError::bad_request("Invalid current password"));
        }
    }

    let mut new_email = None;
    let mut new_pwd_hash = None;

    if let Some(ref email) = payload.email
        && email != &user.email
    {
        if UserRepo::find_by_email(&storage.db_pool, email)
            .await?
            .is_some()
        {
            return Err(HttpError::bad_request("Email is already in use"));
        }
        new_email = Some(email.clone());
    }

    if let Some(ref new_pwd) = payload.new_password {
        if new_pwd.len() < 8 {
            return Err(HttpError::bad_request(
                "New password must be at least 8 characters",
            ));
        }
        let confirm_pwd = payload
            .confirm_new_password
            .as_deref()
            .ok_or_else(|| HttpError::bad_request("Confirm new password is required"))?;
        if new_pwd != confirm_pwd {
            return Err(HttpError::bad_request("New passwords do not match"));
        }
        let hashed = hash_password(new_pwd)?;
        new_pwd_hash = Some(hashed);
    }

    let updated_user =
        UserRepo::update_profile(&storage.db_pool, &user.id, new_email, new_pwd_hash).await?;

    let exp = Utc::now().timestamp() as usize + EXP_TIME;
    let token_payload = TokenPayload {
        id: updated_user.id.clone(),
        email: updated_user.email.clone(),
        exp,
    };

    let token = create_jwt(&token_payload, &config.jwt_secret)?;

    Ok(Json(serde_json::json!({
        "token": token,
        "user": updated_user,
    })))
}
