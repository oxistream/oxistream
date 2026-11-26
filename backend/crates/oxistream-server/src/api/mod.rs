use axum::routing::get;
use axum::{Json, Router};
use utoipa::OpenApi;
use utoipa::openapi::OpenApi as Spec;
use utoipa_axum::router::OpenApiRouter;
use utoipa_scalar::{Scalar, Servable};

use crate::AppState;

mod v0;

#[derive(OpenApi)]
#[openapi(info(
    title = "Oxistream API",
    description = "HTTP API of the Oxistream server."
))]
struct ApiDoc;

fn split() -> (Router<AppState>, Spec) {
    OpenApiRouter::with_openapi(ApiDoc::openapi())
        .nest("/api/v0", v0::router())
        .split_for_parts()
}

/// The `OpenAPI` document, built without any runtime state.
pub fn spec() -> Spec {
    split().1
}

pub fn spec_json() -> String {
    let mut json = spec()
        .to_pretty_json()
        .expect("OpenAPI spec serializes to JSON");
    json.push('\n');
    json
}

pub fn router() -> Router<AppState> {
    let (router, spec) = split();
    let raw = spec.clone();
    router
        .route("/api/openapi.json", get(move || async move { Json(raw) }))
        .merge(Scalar::with_url("/api/docs", spec))
}
