//! Fallback for unmatched routes: serves the embedded SPA when built with `embed-frontend`.

use axum::http::{Method, StatusCode, Uri};
use axum::response::{IntoResponse, Response};

const BACKEND_PREFIXES: &[&str] = &["/api", "/proxy"];

pub async fn fallback(method: Method, uri: Uri) -> Response {
    let path = uri.path();
    let is_backend = BACKEND_PREFIXES.iter().any(|prefix| {
        path.strip_prefix(prefix)
            .is_some_and(|rest| rest.is_empty() || rest.starts_with('/'))
    });

    if is_backend || !(method == Method::GET || method == Method::HEAD) {
        return StatusCode::NOT_FOUND.into_response();
    }

    serve(path)
}

#[cfg(not(feature = "embed-frontend"))]
fn serve(_path: &str) -> Response {
    StatusCode::NOT_FOUND.into_response()
}

#[cfg(feature = "embed-frontend")]
fn serve(path: &str) -> Response {
    embedded::serve(path)
}

#[cfg(feature = "embed-frontend")]
mod embedded {
    use axum::http::{StatusCode, header};
    use axum::response::{IntoResponse, Response};
    use rust_embed::{EmbeddedFile, RustEmbed};

    #[derive(RustEmbed)]
    #[folder = "../../../frontend/build"]
    #[allow_missing = true]
    struct Assets;

    const IMMUTABLE_PREFIX: &str = "_app/immutable/";

    pub fn serve(path: &str) -> Response {
        let key = path.trim_start_matches('/');
        if let Some(file) = Assets::get(key).filter(|_| !key.is_empty()) {
            return respond(key, file);
        }
        // Missing build assets must not fall back to HTML, or stale chunks fail confusingly.
        if key.starts_with("_app/") {
            return StatusCode::NOT_FOUND.into_response();
        }
        match Assets::get("index.html") {
            Some(file) => respond("index.html", file),
            None => StatusCode::NOT_FOUND.into_response(),
        }
    }

    fn respond(key: &str, file: EmbeddedFile) -> Response {
        let cache_control = if key.starts_with(IMMUTABLE_PREFIX) {
            "public, max-age=31536000, immutable"
        } else {
            "no-cache"
        };
        (
            [
                (header::CONTENT_TYPE, file.metadata.mimetype().to_owned()),
                (header::CACHE_CONTROL, cache_control.to_owned()),
            ],
            file.data,
        )
            .into_response()
    }
}
