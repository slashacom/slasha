use axum::{
    Json, Router,
    extract::{Path, Query, State},
    http::{HeaderMap, StatusCode, header},
    response::IntoResponse,
    routing::{get, post},
};
use axum_extra::extract::{
    CookieJar,
    cookie::{Cookie, SameSite},
};
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use slasha_db::{
    app::{App, AppVisibility},
    repos::app::AppRepo,
    user::UserRole,
};
use time::Duration;

use crate::{AppState, HttpError, HttpResult, auth, extractors::auth::OptionalAuthUser};

pub fn router() -> Router<AppState> {
    Router::new()
        .route("/check", get(access_check_handler))
        .route("/callback", get(access_callback_handler))
        .route("/{slug}/verify", post(verify_app_access))
}

#[derive(Serialize, Deserialize)]
struct AppAccessClaims {
    app_id: String,
    version_hash: String,
    exp: usize,
}

fn compute_visibility_hash(app: &App) -> String {
    let payload = format!(
        "{}:{}",
        app.visibility,
        app.visibility_password_hash.as_deref().unwrap_or("")
    );
    hex::encode(Sha256::digest(payload.as_bytes()))
}

fn create_access_cookie_token(app: &App, secret: &str) -> anyhow::Result<String> {
    let exp = (chrono::Utc::now() + chrono::Duration::days(30)).timestamp() as usize;
    let claims = AppAccessClaims {
        app_id: app.id.clone(),
        version_hash: compute_visibility_hash(app),
        exp,
    };
    let key = jsonwebtoken::EncodingKey::from_secret(secret.as_bytes());
    jsonwebtoken::encode(&jsonwebtoken::Header::default(), &claims, &key)
        .map_err(anyhow::Error::from)
}

fn has_valid_access_cookie(jar: &CookieJar, app: &App, secret: &str) -> bool {
    let cookie_name = format!("slasha_app_access_{}", app.id);
    let Some(cookie) = jar.get(&cookie_name) else {
        return false;
    };

    let key = jsonwebtoken::DecodingKey::from_secret(secret.as_bytes());
    let mut val = jsonwebtoken::Validation::default();
    val.validate_exp = true;
    if let Ok(data) = jsonwebtoken::decode::<AppAccessClaims>(cookie.value(), &key, &val)
        && data.claims.app_id == app.id
        && data.claims.version_hash == compute_visibility_hash(app)
    {
        return true;
    }

    false
}

async fn validate_return_to_for_app(
    pool: &slasha_db::DbPool,
    return_to: &str,
    app: &App,
    platform_domain: &str,
) -> Option<String> {
    let trimmed = return_to.trim();
    if trimmed.is_empty() || trimmed.contains('\r') || trimmed.contains('\n') {
        return None;
    }

    if trimmed.starts_with('/') && !trimmed.starts_with("//") && !trimmed.starts_with("/\\") {
        return Some(trimmed.to_string());
    }

    if let Ok(url) = reqwest::Url::parse(trimmed)
        && let Some(raw_host) = url.host_str()
    {
        let host = raw_host.trim_matches('[').trim_matches(']');
        if let Ok(Some(found_app)) = AppRepo::find_by_host(pool, host, platform_domain).await
            && found_app.id == app.id
        {
            return Some(trimmed.to_string());
        }
    }

    None
}

async fn access_check_handler(
    State(state): State<AppState>,
    OptionalAuthUser(user): OptionalAuthUser,
    jar: CookieJar,
    headers: HeaderMap,
) -> HttpResult<impl IntoResponse> {
    let raw_host = headers
        .get("x-forwarded-host")
        .or_else(|| headers.get("host"))
        .and_then(|h| h.to_str().ok())
        .unwrap_or_default();

    let host = raw_host.split(':').next().unwrap_or(raw_host).trim();

    let Some(app) =
        AppRepo::find_by_host(&state.storage.db_pool, host, &state.config.platform_domain)
            .await
            .map_err(HttpError::internal)?
    else {
        tracing::warn!(host = %host, "forward_auth request for unrecognised host");
        return Ok(StatusCode::OK.into_response());
    };

    match app.visibility {
        AppVisibility::Public => Ok(StatusCode::OK.into_response()),

        AppVisibility::Password => {
            if has_valid_access_cookie(&jar, &app, &state.config.jwt_secret) {
                return Ok(StatusCode::OK.into_response());
            }
            Ok(redirect_to_access_page(
                &headers,
                &state.config.platform_domain,
                &app.slug,
            ))
        }

        AppVisibility::Private => {
            if has_valid_access_cookie(&jar, &app, &state.config.jwt_secret) {
                return Ok(StatusCode::OK.into_response());
            }

            if let Some(ref u) = user {
                if u.role == UserRole::Admin {
                    return Ok(StatusCode::OK.into_response());
                }
                let is_member = AppRepo::find_membership(&state.storage.db_pool, &app.id, &u.id)
                    .await
                    .map_err(HttpError::internal)?
                    .is_some();

                if is_member {
                    return Ok(StatusCode::OK.into_response());
                }
            }
            Ok(redirect_to_access_page(
                &headers,
                &state.config.platform_domain,
                &app.slug,
            ))
        }
    }
}

fn redirect_to_access_page(
    headers: &HeaderMap,
    platform_domain: &str,
    slug: &str,
) -> axum::response::Response {
    let proto = headers
        .get("x-forwarded-proto")
        .and_then(|h| h.to_str().ok())
        .unwrap_or("https");

    let host = headers
        .get("x-forwarded-host")
        .or_else(|| headers.get("host"))
        .and_then(|h| h.to_str().ok())
        .unwrap_or_default();

    let uri = headers
        .get("x-forwarded-uri")
        .and_then(|h| h.to_str().ok())
        .unwrap_or("/");

    let return_to = urlencoding::encode(&format!("{}://{}{}", proto, host, uri)).into_owned();
    let location = format!(
        "{}://{}/apps/{}/access?return_to={}",
        proto, platform_domain, slug, return_to
    );

    (StatusCode::FOUND, [(header::LOCATION, location)]).into_response()
}

#[derive(Deserialize)]
struct CallbackQuery {
    ticket: String,
    return_to: Option<String>,
}

async fn access_callback_handler(
    State(state): State<AppState>,
    jar: CookieJar,
    Query(query): Query<CallbackQuery>,
) -> HttpResult<impl IntoResponse> {
    let app_id = state
        .runtime
        .consume_access_ticket(&query.ticket)
        .await
        .ok_or_else(|| HttpError::bad_request("Invalid or expired access ticket"))?;

    let app = AppRepo::find_by_id(&state.storage.db_pool, &app_id)
        .await
        .map_err(HttpError::internal)?;

    let token = create_access_cookie_token(&app, &state.config.jwt_secret)?;

    let cookie_name = format!("slasha_app_access_{}", app_id);
    let cookie = Cookie::build((cookie_name, token))
        .path("/")
        .max_age(Duration::days(30))
        .http_only(true)
        .same_site(SameSite::Lax)
        .build();

    let raw_return_to = query.return_to.as_deref().unwrap_or("/");
    let return_to = validate_return_to_for_app(
        &state.storage.db_pool,
        raw_return_to,
        &app,
        &state.config.platform_domain,
    )
    .await
    .ok_or_else(|| HttpError::bad_request("Invalid return URL"))?;

    tracing::info!(app_id = %app_id, "app access ticket consumed and cookie granted");

    Ok((
        jar.add(cookie),
        (StatusCode::FOUND, [(header::LOCATION, return_to)]),
    )
        .into_response())
}

#[derive(Deserialize)]
struct VerifyAccessReq {
    password: Option<String>,
}

async fn verify_app_access(
    State(state): State<AppState>,
    OptionalAuthUser(user): OptionalAuthUser,
    Path(slug): Path<String>,
    Json(payload): Json<VerifyAccessReq>,
) -> HttpResult<impl IntoResponse> {
    let app = AppRepo::find_by_slug(&state.storage.db_pool, &slug)
        .await
        .map_err(HttpError::internal)?
        .ok_or_else(|| HttpError::not_found("App not found"))?;

    match app.visibility {
        AppVisibility::Password => {
            let password = payload.password.unwrap_or_default();
            if password.is_empty() {
                return Err(HttpError::bad_request("Password is required"));
            }

            let hash = app.visibility_password_hash.as_deref().ok_or_else(|| {
                HttpError::internal(anyhow::anyhow!("App password is not configured"))
            })?;

            let valid = auth::verify_password(&password, hash)
                .map_err(|_| HttpError::bad_request("Incorrect password"))?;

            if !valid {
                return Err(HttpError::bad_request("Incorrect password"));
            }
        }
        AppVisibility::Private => {
            let u = user.ok_or_else(HttpError::unauthorized)?;
            let is_allowed = u.role == UserRole::Admin
                || AppRepo::find_membership(&state.storage.db_pool, &app.id, &u.id)
                    .await
                    .map_err(HttpError::internal)?
                    .is_some();

            if !is_allowed {
                return Err(HttpError::forbidden("You are not a member of this app"));
            }
        }

        _ => {}
    }

    let ticket = state.runtime.create_access_ticket(&app.id).await;

    tracing::info!(app_slug = %app.slug, app_id = %app.id, "app access verified, ticket created");

    Ok(Json(serde_json::json!({
        "ticket": ticket,
        "app_id": app.id,
    })))
}
