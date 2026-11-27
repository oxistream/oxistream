use axum::Json;
use axum::extract::{Path, State};
use axum::http::{HeaderMap, StatusCode, header};
use axum::response::{IntoResponse, Response};
use oxistream_db::FeedFilter;
use reqwest::Url;
use serde::Serialize;
use time::OffsetDateTime;
use utoipa::ToSchema;
use utoipa_axum::router::OpenApiRouter;
use utoipa_axum::routes;

use crate::AppState;
use crate::api::error::{ApiError, ErrorBody, ErrorCode};
use crate::atom::{self, AtomFeed};
use crate::auth::{CurrentUser, cookie};

/// The token is the only credential, so this prefix is exempt from `OXISTREAM_REQUIRE_LOGIN`
/// and redacted in request logs.
pub const ATOM_FEED_PREFIX: &str = "/api/v0/feed/atom/";
const ATOM_FEED_LIMIT: u32 = 50;
const ATOM_FEED_CACHE: &str = "private, max-age=300";

pub fn router() -> OpenApiRouter<AppState> {
    OpenApiRouter::new()
        .routes(routes!(
            get_feed_token,
            create_feed_token,
            delete_feed_token
        ))
        .routes(routes!(atom_feed))
}

#[derive(Debug, Serialize, ToSchema)]
pub struct FeedTokenStatus {
    /// When the current feed URL was created; `null` without one.
    #[serde(with = "time::serde::rfc3339::option")]
    #[schema(required = true)]
    pub created_at: Option<OffsetDateTime>,
}

#[derive(Debug, Serialize, ToSchema)]
pub struct CreatedFeedToken {
    /// The Atom feed URL. Only the token's hash is stored, so it is shown only once.
    pub url: String,
    #[serde(with = "time::serde::rfc3339")]
    pub created_at: OffsetDateTime,
}

/// Scheme and host this instance is reached at: `OXISTREAM_PUBLIC_URL`, else the request's
/// `Origin`, else its `Host`.
fn instance_origin(state: &AppState, headers: &HeaderMap) -> Option<String> {
    let config = state.auth.config();
    if let Some(origin) = &config.public_origin {
        return Some(origin.clone());
    }
    let header = |name: &str| headers.get(name).and_then(|value| value.to_str().ok());
    if let Some(origin) = header(header::ORIGIN.as_str())
        .and_then(|origin| Url::parse(origin).ok())
        .filter(|url| url.origin().is_tuple())
    {
        return Some(origin.origin().ascii_serialization());
    }
    let scheme = match header("x-forwarded-proto") {
        Some(proto @ ("http" | "https")) => proto,
        _ if config.cookie_secure => "https",
        _ => "http",
    };
    let url = Url::parse(&format!("{scheme}://{}", header(header::HOST.as_str())?)).ok()?;
    (url.path() == "/" && url.origin().is_tuple()).then(|| url.origin().ascii_serialization())
}

fn not_found() -> ApiError {
    ApiError::new(StatusCode::NOT_FOUND, ErrorCode::NotFound, "not found")
}

/// Whether the signed-in user has a feed URL.
#[utoipa::path(
    get,
    path = "/settings/feed-token",
    tag = "settings",
    security(("session" = [])),
    responses(
        (status = OK, body = FeedTokenStatus),
        (status = UNAUTHORIZED, description = "Not signed in", body = ErrorBody),
    )
)]
async fn get_feed_token(
    State(state): State<AppState>,
    CurrentUser(user): CurrentUser,
) -> Result<Json<FeedTokenStatus>, ApiError> {
    Ok(Json(FeedTokenStatus {
        created_at: state.settings.feed_token_created_at(user.id).await?,
    }))
}

/// Create a personal Atom feed URL of the subscription feed; an existing one stops working.
#[utoipa::path(
    post,
    path = "/settings/feed-token",
    tag = "settings",
    security(("session" = [])),
    responses(
        (status = CREATED, body = CreatedFeedToken),
        (status = UNAUTHORIZED, description = "Not signed in", body = ErrorBody),
        (status = FORBIDDEN, description = "Foreign origin", body = ErrorBody),
    )
)]
async fn create_feed_token(
    State(state): State<AppState>,
    CurrentUser(user): CurrentUser,
    headers: HeaderMap,
) -> Result<(StatusCode, Json<CreatedFeedToken>), ApiError> {
    let origin = instance_origin(&state, &headers)
        .ok_or_else(|| ApiError::bad_request("cannot tell this instance's URL"))?;
    let (token, hash) = cookie::new_token();
    let created_at = state.settings.replace_feed_token(user.id, &hash).await?;
    Ok((
        StatusCode::CREATED,
        Json(CreatedFeedToken {
            url: format!("{origin}{ATOM_FEED_PREFIX}{token}"),
            created_at,
        }),
    ))
}

/// Revoke the signed-in user's feed URL.
#[utoipa::path(
    delete,
    path = "/settings/feed-token",
    tag = "settings",
    security(("session" = [])),
    responses(
        (status = NO_CONTENT, description = "Revoked, or there was none"),
        (status = UNAUTHORIZED, description = "Not signed in", body = ErrorBody),
        (status = FORBIDDEN, description = "Foreign origin", body = ErrorBody),
    )
)]
async fn delete_feed_token(
    State(state): State<AppState>,
    CurrentUser(user): CurrentUser,
) -> Result<StatusCode, ApiError> {
    state.settings.delete_feed_token(user.id).await?;
    Ok(StatusCode::NO_CONTENT)
}

/// The newest videos of a user's subscriptions as Atom. The token from
/// `POST /settings/feed-token` authenticates; no session is needed.
#[utoipa::path(
    get,
    path = "/feed/atom/{token}",
    tag = "subscriptions",
    params(("token" = String, Path, description = "Feed token")),
    responses(
        (status = OK, description = "Atom feed", content_type = "application/atom+xml", body = String),
        (status = NOT_FOUND, description = "Unknown or revoked token, or disabled user", body = ErrorBody),
    )
)]
async fn atom_feed(
    State(state): State<AppState>,
    Path(token): Path<String>,
    headers: HeaderMap,
) -> Result<Response, ApiError> {
    let hash = cookie::hash_token(&token).ok_or_else(not_found)?;
    let user_id = state
        .settings
        .feed_token_user(&hash)
        .await?
        .ok_or_else(not_found)?;
    let origin = instance_origin(&state, &headers)
        .ok_or_else(|| ApiError::bad_request("cannot tell this instance's URL"))?;
    let videos = state
        .subscriptions
        .feed(user_id, FeedFilter::All, None, ATOM_FEED_LIMIT)
        .await?;
    let body = AtomFeed {
        id: &format!("urn:uuid:{user_id}"),
        title: "Subscriptions · Oxistream",
        self_url: &format!("{origin}{ATOM_FEED_PREFIX}{token}"),
        origin: &origin,
        videos: &videos,
        image_url: &|url: &str| state.image_url(url),
    }
    .render();
    Ok((
        [
            (header::CONTENT_TYPE, atom::CONTENT_TYPE),
            (header::CACHE_CONTROL, ATOM_FEED_CACHE),
        ],
        body,
    )
        .into_response())
}
