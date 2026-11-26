use axum::Json;
use axum::extract::{Path, State};
use axum::http::StatusCode;
use axum::response::{IntoResponse, Response};
use oxistream_db::FollowedPlaylist as DbFollowedPlaylist;
use serde::Serialize;
use time::OffsetDateTime;
use utoipa::ToSchema;
use utoipa_axum::router::OpenApiRouter;
use utoipa_axum::routes;

use crate::AppState;
use crate::api::dto::{ChannelRef, Thumbnail};
use crate::api::error::{ApiError, ErrorBody, ErrorCode};
use crate::auth::CurrentUser;
use crate::youtube;

pub fn router() -> OpenApiRouter<AppState> {
    OpenApiRouter::new()
        .routes(routes!(list_follows))
        .routes(routes!(follow_state, follow, unfollow))
        .routes(routes!(mark_seen))
}

/// A YouTube playlist the signed-in user follows, as last fetched by the feed worker.
#[derive(Debug, Serialize, ToSchema)]
pub struct FollowedPlaylist {
    /// YouTube playlist ID, used in `/playlist?list=`.
    pub id: String,
    /// `null` until the playlist was fetched once, shortly after the first follow.
    pub title: Option<String>,
    /// Absent for auto-generated lists like albums; `avatar` is always empty.
    pub channel: Option<ChannelRef>,
    pub thumbnails: Vec<Thumbnail>,
    /// As reported by YouTube.
    pub video_count: Option<u64>,
    /// Videos added since the user last opened the playlist. Only the first 500 videos of a
    /// playlist are tracked.
    pub new_video_count: u32,
    /// The last fetch failed, e.g. because the playlist was deleted or made private.
    pub unavailable: bool,
    #[serde(with = "time::serde::rfc3339")]
    pub followed_at: OffsetDateTime,
    #[serde(with = "time::serde::rfc3339::option")]
    pub fetched_at: Option<OffsetDateTime>,
}

#[derive(Debug, Serialize, ToSchema)]
pub struct FollowState {
    pub following: bool,
}

impl FollowedPlaylist {
    fn from_db(follow: DbFollowedPlaylist, img: &impl Fn(&str) -> String) -> Self {
        let p = follow.playlist;
        Self {
            channel: p
                .channel_id
                .zip(p.channel_name)
                .map(|(id, name)| ChannelRef {
                    id,
                    name,
                    avatar: Vec::new(),
                    subscriber_count: None,
                    verified: false,
                }),
            thumbnails: p
                .thumbnails
                .iter()
                .map(|i| Thumbnail {
                    url: img(&i.url),
                    width: i.width,
                    height: i.height,
                })
                .collect(),
            video_count: p.video_count.and_then(|c| u64::try_from(c).ok()),
            new_video_count: u32::try_from(follow.new_count).unwrap_or(u32::MAX),
            unavailable: p.last_error.is_some(),
            followed_at: follow.followed_at,
            fetched_at: p.fetched_at,
            id: p.id,
            title: p.title,
        }
    }
}

fn check_id(id: &str) -> Result<(), ApiError> {
    youtube::check_playlist_id(id).map_err(ApiError::from)
}

/// YouTube playlists the signed-in user follows, ordered by title.
#[utoipa::path(
    get,
    path = "/playlist-follows",
    tag = "playlists",
    security(("session" = [])),
    responses(
        (status = OK, body = Vec<FollowedPlaylist>),
        (status = UNAUTHORIZED, description = "Not signed in", body = ErrorBody),
    )
)]
async fn list_follows(
    State(state): State<AppState>,
    CurrentUser(user): CurrentUser,
) -> Result<Json<Vec<FollowedPlaylist>>, ApiError> {
    let img = |url: &str| state.image_url(url);
    let list = state.follows.list(user.id).await?;
    Ok(Json(
        list.into_iter()
            .map(|f| FollowedPlaylist::from_db(f, &img))
            .collect(),
    ))
}

/// Whether the signed-in user follows a YouTube playlist.
#[utoipa::path(
    get,
    path = "/playlist-follows/{playlist_id}",
    tag = "playlists",
    security(("session" = [])),
    params(("playlist_id" = String, Path, description = "YouTube playlist ID")),
    responses(
        (status = OK, body = FollowState),
        (status = BAD_REQUEST, description = "Invalid playlist ID", body = ErrorBody),
        (status = UNAUTHORIZED, description = "Not signed in", body = ErrorBody),
        (status = NOT_FOUND, description = "A mix (`reason` is `mix`)", body = ErrorBody),
    )
)]
async fn follow_state(
    State(state): State<AppState>,
    CurrentUser(user): CurrentUser,
    Path(playlist_id): Path<String>,
) -> Result<Json<FollowState>, ApiError> {
    check_id(&playlist_id)?;
    let following = state.follows.is_following(user.id, &playlist_id).await?;
    Ok(Json(FollowState { following }))
}

/// Follow a YouTube playlist. Idempotent; new playlists are fetched in the background and
/// then kept up to date by the feed worker.
#[utoipa::path(
    put,
    path = "/playlist-follows/{playlist_id}",
    tag = "playlists",
    security(("session" = [])),
    params(("playlist_id" = String, Path, description = "YouTube playlist ID")),
    responses(
        (status = CREATED, description = "Followed", body = FollowedPlaylist),
        (status = OK, description = "Was already followed", body = FollowedPlaylist),
        (status = BAD_REQUEST, description = "Invalid playlist ID", body = ErrorBody),
        (status = UNAUTHORIZED, description = "Not signed in", body = ErrorBody),
        (status = FORBIDDEN, description = "Foreign origin", body = ErrorBody),
        (status = NOT_FOUND, description = "Mixes cannot be followed (`reason` is `mix`)", body = ErrorBody),
    )
)]
async fn follow(
    State(state): State<AppState>,
    CurrentUser(user): CurrentUser,
    Path(playlist_id): Path<String>,
) -> Result<Response, ApiError> {
    check_id(&playlist_id)?;
    let (follow, created) = state.follows.follow(user.id, &playlist_id).await?;
    if created {
        state.feed.request_playlist_fetch(&playlist_id);
    }
    let status = if created {
        StatusCode::CREATED
    } else {
        StatusCode::OK
    };
    let body = FollowedPlaylist::from_db(follow, &|url| state.image_url(url));
    Ok((status, Json(body)).into_response())
}

/// Stop following a YouTube playlist. Succeeds if not followed, too.
#[utoipa::path(
    delete,
    path = "/playlist-follows/{playlist_id}",
    tag = "playlists",
    security(("session" = [])),
    params(("playlist_id" = String, Path, description = "YouTube playlist ID")),
    responses(
        (status = NO_CONTENT, description = "Not followed anymore"),
        (status = BAD_REQUEST, description = "Invalid playlist ID", body = ErrorBody),
        (status = UNAUTHORIZED, description = "Not signed in", body = ErrorBody),
        (status = FORBIDDEN, description = "Foreign origin", body = ErrorBody),
        (status = NOT_FOUND, description = "A mix (`reason` is `mix`)", body = ErrorBody),
    )
)]
async fn unfollow(
    State(state): State<AppState>,
    CurrentUser(user): CurrentUser,
    Path(playlist_id): Path<String>,
) -> Result<StatusCode, ApiError> {
    check_id(&playlist_id)?;
    state.follows.unfollow(user.id, &playlist_id).await?;
    Ok(StatusCode::NO_CONTENT)
}

/// Mark the videos of a followed playlist as seen, resetting `new_video_count`.
#[utoipa::path(
    put,
    path = "/playlist-follows/{playlist_id}/seen",
    tag = "playlists",
    security(("session" = [])),
    params(("playlist_id" = String, Path, description = "YouTube playlist ID")),
    responses(
        (status = NO_CONTENT, description = "Marked as seen"),
        (status = BAD_REQUEST, description = "Invalid playlist ID", body = ErrorBody),
        (status = UNAUTHORIZED, description = "Not signed in", body = ErrorBody),
        (status = FORBIDDEN, description = "Foreign origin", body = ErrorBody),
        (status = NOT_FOUND, description = "Not followed", body = ErrorBody),
    )
)]
async fn mark_seen(
    State(state): State<AppState>,
    CurrentUser(user): CurrentUser,
    Path(playlist_id): Path<String>,
) -> Result<StatusCode, ApiError> {
    check_id(&playlist_id)?;
    if state.follows.mark_seen(user.id, &playlist_id).await? {
        Ok(StatusCode::NO_CONTENT)
    } else {
        Err(ApiError::new(
            StatusCode::NOT_FOUND,
            ErrorCode::NotFound,
            "playlist not followed",
        ))
    }
}
