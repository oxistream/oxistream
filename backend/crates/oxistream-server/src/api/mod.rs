use axum::routing::get;
use axum::{Json, Router};
use utoipa::openapi::OpenApi as Spec;
use utoipa::openapi::security::{ApiKey, ApiKeyValue, SecurityScheme};
use utoipa::{Modify, OpenApi};
use utoipa_axum::router::OpenApiRouter;
use utoipa_scalar::{Scalar, Servable};

use crate::AppState;

mod dto;
pub mod error;
mod lang;
pub(crate) mod v0;

pub use v0::feed_token::ATOM_FEED_PREFIX;

#[derive(OpenApi)]
#[openapi(
    info(
        title = "Oxistream API",
        description = "HTTP API of the Oxistream server. State-changing requests must send an \
            `Origin` (or `Referer`) header of this instance."
    ),
    components(schemas(
        v0::rooms::protocol::RoomClientMessage,
        v0::rooms::protocol::RoomServerMessage
    )),
    modifiers(&SessionCookie)
)]
struct ApiDoc;

struct SessionCookie;

impl Modify for SessionCookie {
    fn modify(&self, openapi: &mut Spec) {
        openapi
            .components
            .get_or_insert_default()
            .add_security_scheme(
                "session",
                SecurityScheme::ApiKey(ApiKey::Cookie(ApiKeyValue::new(
                    crate::auth::cookie::SESSION_COOKIE,
                ))),
            );
    }
}

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
