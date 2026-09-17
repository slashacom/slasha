use axum::{extract::Request, middleware::Next, response::Response};

use crate::{HttpResult, extractors::auth::AuthUser};

pub async fn auth_middleware(
    auth: AuthUser,
    mut request: Request,
    next: Next,
) -> HttpResult<Response> {
    request.extensions_mut().insert(auth.0);
    Ok(next.run(request).await)
}
