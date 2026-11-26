use axum::Json;
use axum::extract::{Path, Query, State};
use axum::http::StatusCode;
use axum::response::{IntoResponse, Response};
use oxistream_db::{FeedCursor, FeedVideo, Image, Subscription};
use rustypipe::validate;
use serde::{Deserialize, Serialize};
use time::OffsetDateTime;
use utoipa::{IntoParams, ToSchema};
use utoipa_axum::router::OpenApiRouter;
use utoipa_axum::routes;

use crate::AppState;
use crate::api::dto::{ChannelRef, Thumbnail, VideoSummary};
use crate::api::error::{ApiError, ErrorBody};
use crate::auth::CurrentUser;

const FEED_DEFAULT_LIMIT: u32 = 30;
const FEED_MAX_LIMIT: u32 = 100;

pub fn router() -> OpenApiRouter<AppState> {
    OpenApiRouter::new()
        .routes(routes!(list_subscriptions))
        .routes(routes!(subscription_state, subscribe, unsubscribe))
        .routes(routes!(feed))
}

/// A channel the signed-in user subscribed to.
#[derive(Debug, Serialize, ToSchema)]
pub struct SubscribedChannel {
    pub id: String,
    /// `null` until the channel was fetched once, shortly after the first subscription.
    pub name: Option<String>,
    pub handle: Option<String>,
    pub avatar: Vec<Thumbnail>,
    pub subscriber_count: Option<u64>,
    pub verified: bool,
    #[serde(with = "time::serde::rfc3339")]
    pub subscribed_at: OffsetDateTime,
}

#[derive(Debug, Serialize, ToSchema)]
pub struct SubscriptionState {
    pub subscribed: bool,
}

#[derive(Debug, Serialize, ToSchema)]
pub struct FeedPage {
    /// Newest first; `channel` is always set.
    pub items: Vec<VideoSummary>,
    /// Pass to `continuation` to fetch the next page; absent on the last page.
    pub continuation: Option<String>,
}

fn thumbnails(src: &[Image], img: &impl Fn(&str) -> String) -> Vec<Thumbnail> {
    src.iter()
        .map(|i| Thumbnail {
            url: img(&i.url),
            width: i.width,
            height: i.height,
        })
        .collect()
}

impl SubscribedChannel {
    fn from_db(subscription: Subscription, img: &impl Fn(&str) -> String) -> Self {
        let channel = subscription.channel;
        Self {
            avatar: thumbnails(&channel.avatar, img),
            subscriber_count: channel.subscriber_count.and_then(|c| u64::try_from(c).ok()),
            verified: channel.verified,
            subscribed_at: subscription.subscribed_at,
            id: channel.id,
            name: channel.name,
            handle: channel.handle,
        }
    }
}

fn feed_item(video: FeedVideo, img: &impl Fn(&str) -> String) -> VideoSummary {
    let view_count = video.view_count.and_then(|c| u64::try_from(c).ok());
    VideoSummary {
        thumbnails: thumbnails(&video.thumbnails, img),
        channel: Some(ChannelRef {
            avatar: thumbnails(&video.channel_avatar, img),
            name: video
                .channel_name
                .unwrap_or_else(|| video.channel_id.clone()),
            id: video.channel_id,
            subscriber_count: None,
            verified: video.channel_verified,
        }),
        duration_seconds: video.duration_seconds.and_then(|d| u32::try_from(d).ok()),
        published_at: video.published_at,
        published_text: video.published_text,
        view_count: view_count.filter(|_| !video.is_live),
        viewers: view_count.filter(|_| video.is_live),
        is_live: video.is_live,
        is_short: video.is_short,
        is_upcoming: video.is_upcoming,
        description: None,
        id: video.video_id,
        title: video.title,
    }
}

fn encode_cursor(cursor: &FeedCursor) -> String {
    let micros = cursor.sort_at.unix_timestamp_nanos() / 1000;
    format!("{micros}.{}", cursor.video_id)
}

fn decode_cursor(token: &str) -> Option<FeedCursor> {
    let (micros, video_id) = token.split_once('.')?;
    let micros: i128 = micros.parse().ok()?;
    let sort_at = OffsetDateTime::from_unix_timestamp_nanos(micros.checked_mul(1000)?).ok()?;
    validate::video_id(video_id).ok()?;
    Some(FeedCursor {
        sort_at,
        video_id: video_id.to_owned(),
    })
}

fn check_channel_id(id: &str) -> Result<(), ApiError> {
    validate::channel_id(id).map_err(|_| ApiError::bad_request("invalid channel id"))
}

/// Channels the signed-in user subscribed to, ordered by name.
#[utoipa::path(
    get,
    path = "/subscriptions",
    tag = "subscriptions",
    security(("session" = [])),
    responses(
        (status = OK, body = Vec<SubscribedChannel>),
        (status = UNAUTHORIZED, description = "Not signed in", body = ErrorBody),
    )
)]
async fn list_subscriptions(
    State(state): State<AppState>,
    CurrentUser(user): CurrentUser,
) -> Result<Json<Vec<SubscribedChannel>>, ApiError> {
    let img = |url: &str| state.image_url(url);
    let subscriptions = state.subscriptions.list(user.id).await?;
    Ok(Json(
        subscriptions
            .into_iter()
            .map(|s| SubscribedChannel::from_db(s, &img))
            .collect(),
    ))
}

/// Whether the signed-in user is subscribed to a channel.
#[utoipa::path(
    get,
    path = "/subscriptions/{channel_id}",
    tag = "subscriptions",
    security(("session" = [])),
    params(("channel_id" = String, Path, description = "YouTube channel ID (`UC…`)")),
    responses(
        (status = OK, body = SubscriptionState),
        (status = BAD_REQUEST, description = "Invalid channel ID", body = ErrorBody),
        (status = UNAUTHORIZED, description = "Not signed in", body = ErrorBody),
    )
)]
async fn subscription_state(
    State(state): State<AppState>,
    CurrentUser(user): CurrentUser,
    Path(channel_id): Path<String>,
) -> Result<Json<SubscriptionState>, ApiError> {
    check_channel_id(&channel_id)?;
    let subscribed = state
        .subscriptions
        .is_subscribed(user.id, &channel_id)
        .await?;
    Ok(Json(SubscriptionState { subscribed }))
}

/// Subscribe to a channel. Idempotent; new channels are fetched in the background.
#[utoipa::path(
    put,
    path = "/subscriptions/{channel_id}",
    tag = "subscriptions",
    security(("session" = [])),
    params(("channel_id" = String, Path, description = "YouTube channel ID (`UC…`)")),
    responses(
        (status = CREATED, description = "Subscribed", body = SubscribedChannel),
        (status = OK, description = "Was already subscribed", body = SubscribedChannel),
        (status = BAD_REQUEST, description = "Invalid channel ID", body = ErrorBody),
        (status = UNAUTHORIZED, description = "Not signed in", body = ErrorBody),
        (status = FORBIDDEN, description = "Foreign origin", body = ErrorBody),
    )
)]
async fn subscribe(
    State(state): State<AppState>,
    CurrentUser(user): CurrentUser,
    Path(channel_id): Path<String>,
) -> Result<Response, ApiError> {
    check_channel_id(&channel_id)?;
    let (subscription, created) = state.subscriptions.subscribe(user.id, &channel_id).await?;
    if created {
        state.feed.request_fetch(&channel_id);
    }
    let status = if created {
        StatusCode::CREATED
    } else {
        StatusCode::OK
    };
    let body = SubscribedChannel::from_db(subscription, &|url| state.image_url(url));
    Ok((status, Json(body)).into_response())
}

/// Unsubscribe from a channel. Succeeds if not subscribed, too.
#[utoipa::path(
    delete,
    path = "/subscriptions/{channel_id}",
    tag = "subscriptions",
    security(("session" = [])),
    params(("channel_id" = String, Path, description = "YouTube channel ID (`UC…`)")),
    responses(
        (status = NO_CONTENT, description = "Not subscribed anymore"),
        (status = BAD_REQUEST, description = "Invalid channel ID", body = ErrorBody),
        (status = UNAUTHORIZED, description = "Not signed in", body = ErrorBody),
        (status = FORBIDDEN, description = "Foreign origin", body = ErrorBody),
    )
)]
async fn unsubscribe(
    State(state): State<AppState>,
    CurrentUser(user): CurrentUser,
    Path(channel_id): Path<String>,
) -> Result<StatusCode, ApiError> {
    check_channel_id(&channel_id)?;
    state
        .subscriptions
        .unsubscribe(user.id, &channel_id)
        .await?;
    Ok(StatusCode::NO_CONTENT)
}

#[derive(Debug, Deserialize, IntoParams)]
#[into_params(parameter_in = Query)]
struct FeedParams {
    /// Token from a previous page.
    continuation: Option<String>,
    /// Page size, 1 to 100; default 30.
    limit: Option<u32>,
}

/// Videos of subscribed channels, newest first.
#[utoipa::path(
    get,
    path = "/feed",
    tag = "subscriptions",
    security(("session" = [])),
    params(FeedParams),
    responses(
        (status = OK, body = FeedPage),
        (status = BAD_REQUEST, description = "Invalid continuation or limit", body = ErrorBody),
        (status = UNAUTHORIZED, description = "Not signed in", body = ErrorBody),
    )
)]
async fn feed(
    State(state): State<AppState>,
    CurrentUser(user): CurrentUser,
    Query(params): Query<FeedParams>,
) -> Result<Json<FeedPage>, ApiError> {
    let limit = params.limit.unwrap_or(FEED_DEFAULT_LIMIT);
    if !(1..=FEED_MAX_LIMIT).contains(&limit) {
        return Err(ApiError::bad_request(format!(
            "`limit` must be between 1 and {FEED_MAX_LIMIT}"
        )));
    }
    let after = match params.continuation.as_deref().map(str::trim) {
        None | Some("") => None,
        Some(token) => Some(
            decode_cursor(token).ok_or_else(|| ApiError::bad_request("invalid continuation"))?,
        ),
    };
    let mut videos = state
        .subscriptions
        .feed(user.id, after.as_ref(), limit + 1)
        .await?;
    let continuation = if videos.len() > limit as usize {
        videos.truncate(limit as usize);
        videos.last().map(|v| encode_cursor(&v.cursor()))
    } else {
        None
    };
    let img = |url: &str| state.image_url(url);
    Ok(Json(FeedPage {
        items: videos.into_iter().map(|v| feed_item(v, &img)).collect(),
        continuation,
    }))
}

#[cfg(test)]
mod tests {
    use time::macros::datetime;

    use super::*;

    #[test]
    fn cursor_round_trips() {
        let cursor = FeedCursor {
            sort_at: datetime!(2026-11-26 12:34:56.123456 UTC),
            video_id: "dQw4w9WgXcQ".into(),
        };
        let token = encode_cursor(&cursor);
        assert_eq!(decode_cursor(&token), Some(cursor));
    }

    #[test]
    fn rejects_malformed_cursors() {
        for token in [
            "",
            "x",
            "12",
            "abc.dQw4w9WgXcQ",
            "12.not-a-video-id!",
            "1e99.dQw4w9WgXcQ",
        ] {
            assert_eq!(decode_cursor(token), None, "{token}");
        }
    }
}
