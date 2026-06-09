use axum::{
    body::Body,
    http::{HeaderValue, StatusCode, header},
    response::{IntoResponse, Response},
};
use mime_guess;
use rust_embed::RustEmbed;

#[derive(RustEmbed)]
#[folder = "../../web/build/client"]
pub struct Assets;

pub async fn static_handler(path: Option<axum::extract::Path<String>>) -> impl IntoResponse {
    let path = path
        .map(|p| p.0)
        .unwrap_or_else(|| "index.html".to_string());

    let path = path.trim_start_matches('/');

    // `/` -> the prerendered homepage. Anything else without a matching
    // built asset is a client-side route — serve the SPA fallback (just
    // <FullPageSpinner />) so refreshing on /apps doesn't flash the
    // homepage HTML before the JS bundle hydrates and routes there.
    if path.is_empty() || path == "index.html" {
        return serve_html("index.html").await;
    }

    match Assets::get(path) {
        Some(content) => {
            let mime = mime_guess::from_path(path).first_or_octet_stream();
            Response::builder()
                .header(
                    header::CONTENT_TYPE,
                    HeaderValue::from_str(mime.as_ref()).unwrap(),
                )
                .body(Body::from(content.data))
                .unwrap()
        }
        None => serve_html("__spa-fallback.html").await,
    }
}

async fn serve_html(name: &str) -> Response {
    match Assets::get(name).or_else(|| Assets::get("index.html")) {
        Some(content) => Response::builder()
            .header(header::CONTENT_TYPE, "text/html")
            .body(Body::from(content.data))
            .unwrap(),
        None => (StatusCode::NOT_FOUND, "Index not found").into_response(),
    }
}
