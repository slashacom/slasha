use axum::{
    Json, Router,
    extract::{Path, State},
    middleware::from_fn_with_state,
    response::IntoResponse,
    routing::{delete, get, patch, post},
};
use garde::Validate;
use serde::Deserialize;
use slasha_db::{
    repos::{app::AppRepo, user::UserRepo},
    user::{NewUser, UserChangeset, UserRole},
};

use crate::{
    HttpError, HttpResult,
    auth::hash_password,
    extractors::{ValidatedJson, auth::AuthUser},
    middleware::admin::admin_middleware,
    state::{AppState, Storage},
};

pub fn router(state: AppState) -> Router<AppState> {
    let admin_routes = Router::new()
        .route("/", post(create_user))
        .route("/{id}", get(get_user))
        .route("/{id}", patch(update_user))
        .route("/{id}", delete(delete_user))
        .route_layer(from_fn_with_state(state, admin_middleware));

    Router::new()
        .route("/", get(list_users))
        .merge(admin_routes)
}

async fn get_user(
    State(storage): State<Storage>,
    Path(id): Path<String>,
) -> HttpResult<impl IntoResponse> {
    let user = UserRepo::find_by_id(&storage.db_pool, &id).await?;

    Ok(Json(serde_json::json!({
        "user": user,
    })))
}

async fn list_users(
    _auth: AuthUser,
    State(storage): State<Storage>,
) -> HttpResult<impl IntoResponse> {
    let all_users = UserRepo::list(&storage.db_pool).await?;

    Ok(Json(serde_json::json!({
        "users": all_users,
    })))
}

#[derive(Deserialize, Validate)]
struct CreateUserReq {
    #[serde(deserialize_with = "crate::routing::api::deserialize::trim_string")]
    #[garde(email)]
    email: String,
    #[garde(length(min = 8))]
    password: String,
    #[garde(skip)]
    role: UserRole,
}

async fn create_user(
    State(storage): State<Storage>,
    ValidatedJson(payload): ValidatedJson<CreateUserReq>,
) -> HttpResult<impl IntoResponse> {
    let hashed = hash_password(&payload.password)?;
    let new_user = NewUser {
        email: payload.email,
        password_hash: hashed,
        role: payload.role,
    };

    let new_user = UserRepo::create(&storage.db_pool, new_user).await?;

    Ok(Json(serde_json::json!({
        "user": new_user,
    })))
}

#[derive(Deserialize, Validate)]
struct UpdateUserReq {
    #[serde(
        default,
        deserialize_with = "crate::routing::api::deserialize::trim_optional_string"
    )]
    #[garde(inner(email))]
    email: Option<String>,
    #[garde(skip)]
    role: Option<UserRole>,
    #[garde(inner(length(min = 8)))]
    password: Option<String>,
}

async fn update_user(
    State(storage): State<Storage>,
    Path(id): Path<String>,
    ValidatedJson(payload): ValidatedJson<UpdateUserReq>,
) -> HttpResult<impl IntoResponse> {
    let user = UserRepo::find_by_id(&storage.db_pool, &id).await?;

    if let Some(new_role) = payload.role
        && user.role != new_role
    {
        if user.role == UserRole::Admin && new_role == UserRole::User {
            let admin_count = UserRepo::admin_count(&storage.db_pool).await?;
            if admin_count == 1 {
                return Err(HttpError::bad_request(
                    "There needs to be at least one admin user!",
                ));
            }
        }
        AppRepo::remove_non_owner_memberships_for_user(&storage.db_pool, &id).await?;
    }

    let password_hash = payload.password.map(|p| hash_password(&p)).transpose()?;

    let updated_user = UserRepo::update(
        &storage.db_pool,
        &id,
        UserChangeset {
            email: payload.email,
            role: payload.role,
            password_hash,
        },
    )
    .await?;

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
