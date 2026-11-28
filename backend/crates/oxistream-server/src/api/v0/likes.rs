use axum::Json;
use axum::extract::{Path, Query, State};
use axum::http::StatusCode;
use axum::response::{IntoResponse, Response};
use oxistream_db::{LikeCursor, LikedVideo};
use serde::{Deserialize, Serialize};
use time::OffsetDateTime;
use utoipa::{IntoParams, ToSchema};
use utoipa_axum::router::OpenApiRouter;
use utoipa_axum::routes;

use super::cursor;
use super::playlists::{cached_video_summary, check_video_id, ensure_cached};
use crate::AppState;
use crate::api::dto::VideoSummary;
use crate::api::error::{ApiError, ErrorBody};
use crate::api::lang::ContentLanguage;
use crate::auth::CurrentUser;

const DEFAULT_LIMIT: u32 = 50;
const MAX_LIMIT: u32 = 100;

pub fn router() -> OpenApiRouter<AppState> {
    OpenApiRouter::new()
        .routes(routes!(list_likes))
        .routes(routes!(like_state, like, unlike))
}

/// Whether the signed-in user liked a video.
#[derive(Debug, Serialize, ToSchema)]
pub struct LikeState {
    pub liked: bool,
    /// `null` unless liked.
    #[serde(with = "time::serde::rfc3339::option")]
    pub liked_at: Option<OffsetDateTime>,
}

#[derive(Debug, Serialize, ToSchema)]
pub struct LikedVideoEntry {
    /// From the metadata cache: `channel` is always set; view counts are not.
    pub video: VideoSummary,
    #[serde(with = "time::serde::rfc3339")]
    pub liked_at: OffsetDateTime,
}

#[derive(Debug, Serialize, ToSchema)]
pub struct LikedVideosPage {
    /// Most recently liked first.
    pub items: Vec<LikedVideoEntry>,
    /// Number of liked videos in all pages.
    pub total: u32,
    /// Pass to `continuation` to fetch the next page; absent on the last page.
    pub continuation: Option<String>,
}

#[derive(Debug, Deserialize, IntoParams)]
#[into_params(parameter_in = Query)]
struct ListParams {
    /// Token from a previous page.
    continuation: Option<String>,
    /// Page size, 1 to 100; default 50.
    limit: Option<u32>,
}

fn entry(liked: LikedVideo, img: &impl Fn(&str) -> String) -> LikedVideoEntry {
    LikedVideoEntry {
        liked_at: liked.liked_at,
        video: cached_video_summary(liked.video, img),
    }
}

/// List liked videos
///
/// Most recently liked first. Likes stay on this instance and are never sent to YouTube.
#[utoipa::path(
    get,
    path = "/likes",
    tag = "likes",
    security(("session" = [])),
    params(ListParams),
    responses(
        (status = OK, body = LikedVideosPage),
        (status = BAD_REQUEST, description = "Invalid continuation or limit", body = ErrorBody),
        (status = UNAUTHORIZED, description = "Not signed in", body = ErrorBody),
    )
)]
async fn list_likes(
    State(state): State<AppState>,
    CurrentUser(user): CurrentUser,
    Query(params): Query<ListParams>,
) -> Result<Json<LikedVideosPage>, ApiError> {
    let limit = params.limit.unwrap_or(DEFAULT_LIMIT);
    if !(1..=MAX_LIMIT).contains(&limit) {
        return Err(ApiError::bad_request(format!(
            "`limit` must be between 1 and {MAX_LIMIT}"
        )));
    }
    let after = match params.continuation.as_deref().map(str::trim) {
        None | Some("") => None,
        Some(token) => {
            let (liked_at, video_id) = cursor::decode(token)
                .ok_or_else(|| ApiError::bad_request("invalid continuation"))?;
            Some(LikeCursor { liked_at, video_id })
        }
    };
    let (likes, total) = tokio::try_join!(
        state.likes.list(user.id, after.as_ref(), limit + 1),
        state.likes.count(user.id)
    )?;
    let mut likes = likes;
    let continuation = if likes.len() > limit as usize {
        likes.truncate(limit as usize);
        likes.last().map(|l| {
            let c = l.cursor();
            cursor::encode(c.liked_at, &c.video_id)
        })
    } else {
        None
    };
    let img = |url: &str| state.image_url(url);
    Ok(Json(LikedVideosPage {
        items: likes.into_iter().map(|l| entry(l, &img)).collect(),
        total: u32::try_from(total).unwrap_or(u32::MAX),
        continuation,
    }))
}

/// Get like state
///
/// Whether the signed-in user liked a video.
#[utoipa::path(
    get,
    path = "/likes/{video_id}",
    tag = "likes",
    security(("session" = [])),
    params(("video_id" = String, Path, description = "YouTube video ID")),
    responses(
        (status = OK, body = LikeState),
        (status = BAD_REQUEST, description = "Invalid video ID", body = ErrorBody),
        (status = UNAUTHORIZED, description = "Not signed in", body = ErrorBody),
    )
)]
async fn like_state(
    State(state): State<AppState>,
    CurrentUser(user): CurrentUser,
    Path(video_id): Path<String>,
) -> Result<Json<LikeState>, ApiError> {
    check_video_id(&video_id)?;
    let liked_at = state.likes.liked_at(user.id, &video_id).await?;
    Ok(Json(LikeState {
        liked: liked_at.is_some(),
        liked_at,
    }))
}

/// Like video
///
/// Shorts included. Idempotent; liking again keeps the original date.
#[utoipa::path(
    put,
    path = "/likes/{video_id}",
    tag = "likes",
    security(("session" = [])),
    params(
        ("video_id" = String, Path, description = "YouTube video ID"),
        ("Accept-Language" = Option<String>, Header, description = "Content language"),
    ),
    responses(
        (status = CREATED, description = "Liked", body = LikeState),
        (status = OK, description = "Was already liked", body = LikeState),
        (status = BAD_REQUEST, description = "Invalid video ID", body = ErrorBody),
        (status = UNAUTHORIZED, description = "Not signed in", body = ErrorBody),
        (status = FORBIDDEN, description = "Foreign origin", body = ErrorBody),
        (status = NOT_FOUND, description = "No such video", body = ErrorBody),
        (status = BAD_GATEWAY, description = "YouTube request failed", body = ErrorBody),
        (status = SERVICE_UNAVAILABLE, description = "Rate limited by YouTube", body = ErrorBody),
    )
)]
async fn like(
    State(state): State<AppState>,
    CurrentUser(user): CurrentUser,
    Path(video_id): Path<String>,
    ContentLanguage(lang): ContentLanguage,
) -> Result<Response, ApiError> {
    check_video_id(&video_id)?;
    Box::pin(ensure_cached(&state, &video_id, lang)).await?;
    let (liked_at, created) = state.likes.like(user.id, &video_id).await?;
    let status = if created {
        StatusCode::CREATED
    } else {
        StatusCode::OK
    };
    let body = LikeState {
        liked: true,
        liked_at: Some(liked_at),
    };
    Ok((status, Json(body)).into_response())
}

/// Remove like
///
/// Succeeds if the video was not liked, too.
#[utoipa::path(
    delete,
    path = "/likes/{video_id}",
    tag = "likes",
    security(("session" = [])),
    params(("video_id" = String, Path, description = "YouTube video ID")),
    responses(
        (status = NO_CONTENT, description = "Not liked anymore"),
        (status = BAD_REQUEST, description = "Invalid video ID", body = ErrorBody),
        (status = UNAUTHORIZED, description = "Not signed in", body = ErrorBody),
        (status = FORBIDDEN, description = "Foreign origin", body = ErrorBody),
    )
)]
async fn unlike(
    State(state): State<AppState>,
    CurrentUser(user): CurrentUser,
    Path(video_id): Path<String>,
) -> Result<StatusCode, ApiError> {
    check_video_id(&video_id)?;
    state.likes.unlike(user.id, &video_id).await?;
    Ok(StatusCode::NO_CONTENT)
}
