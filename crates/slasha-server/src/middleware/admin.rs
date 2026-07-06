use axum::{extract::Request, middleware::Next, response::Response};
use slasha_db::user::UserRole;

use crate::{HttpError, HttpResult, extractors::auth::AuthUser};

pub async fn admin_middleware(
    auth: AuthUser,
    request: Request,
    next: Next,
) -> HttpResult<Response> {
    if auth.0.role != UserRole::Admin {
        return Err(HttpError::forbidden("Admin access required"));
    }

    Ok(next.run(request).await)
}
