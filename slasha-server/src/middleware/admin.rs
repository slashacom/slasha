use axum::{extract::Request, middleware::Next, response::Response};

use crate::{
    error::{Error, Result},
    extractors::auth::AuthUser,
};

pub async fn admin_middleware(auth: AuthUser, request: Request, next: Next) -> Result<Response> {
    if auth.0.role != "admin" {
        return Err(Error::Forbidden("Admin access required".into()));
    }

    Ok(next.run(request).await)
}
