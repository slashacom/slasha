use std::time::Duration;

use axum::{
    Json, Router,
    extract::State,
    middleware,
    response::IntoResponse,
    routing::{get, post},
};
use chrono::Utc;
use garde::Validate;
use crate::routing::api::validation::not_empty;
use serde::Deserialize;
use slasha_db::{
    repos::user::UserRepo,
    user::{NewUser, UserChangeset, UserRole},
};

use crate::{
    HttpError, HttpResult,
    auth::{TokenPayload, create_jwt, hash_password, verify_password},
    extractors::{ValidatedJson, auth::AuthUser},
    middleware::rate_limit::{RateLimit, RateLimiter, rate_limit_middleware},
    routing::api::deserialize::trim_string,
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

#[derive(Deserialize, Validate)]
pub struct SignupReq {
    #[serde(deserialize_with = "trim_string")]
    #[garde(email)]
    pub email: String,
    #[garde(length(min = 8))]
    pub password: String,
    #[garde(skip)]
    pub confirm_password: String,
}

async fn signup(
    State(storage): State<Storage>,
    State(config): State<Config>,
    ValidatedJson(payload): ValidatedJson<SignupReq>,
) -> HttpResult<impl IntoResponse> {
    let admin_count = UserRepo::admin_count(&storage.db_pool).await?;

    if admin_count > 0 {
        return Err(HttpError::bad_request("An admin already exists"));
    }

    if payload.password != payload.confirm_password {
        return Err(HttpError::bad_request("Passwords do not match"));
    }

    let hashed = hash_password(&payload.password)?;
    let new_user = NewUser {
        email: payload.email.clone(),
        password_hash: hashed,
        role: UserRole::Admin,
    };

    let created_user = UserRepo::create(&storage.db_pool, new_user).await?;

    let exp = Utc::now().timestamp() as usize + EXP_TIME;
    let token_payload = TokenPayload {
        id: created_user.id.clone(),
        email: created_user.email.clone(),
        exp,
    };

    let token = create_jwt(&token_payload, &config.jwt_secret)?;

    Ok(Json(serde_json::json!({
        "token": token,
        "user": created_user,
    })))
}

#[derive(Deserialize, Validate)]
pub struct LoginReq {
    #[serde(deserialize_with = "trim_string")]
    #[garde(email)]
    pub email: String,
    #[garde(custom(not_empty))]
    pub password: String,
}

async fn login(
    State(storage): State<Storage>,
    State(config): State<Config>,
    ValidatedJson(payload): ValidatedJson<LoginReq>,
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

#[derive(Deserialize, Validate)]
pub struct UpdateProfileReq {
    #[serde(
        default,
        deserialize_with = "crate::routing::api::deserialize::trim_optional_string"
    )]
    #[garde(inner(email))]
    pub email: Option<String>,
    #[garde(skip)]
    pub current_password: Option<String>,
    #[garde(inner(length(min = 8)))]
    pub new_password: Option<String>,
    #[garde(skip)]
    pub confirm_new_password: Option<String>,
}

async fn update_profile(
    AuthUser(user): AuthUser,
    State(storage): State<Storage>,
    State(config): State<Config>,
    ValidatedJson(payload): ValidatedJson<UpdateProfileReq>,
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

    let updated_user = UserRepo::update(
        &storage.db_pool,
        &user.id,
        UserChangeset {
            email: new_email,
            role: None,
            password_hash: new_pwd_hash,
            updated_at: Utc::now().naive_utc(),
        },
    )
    .await?;

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
