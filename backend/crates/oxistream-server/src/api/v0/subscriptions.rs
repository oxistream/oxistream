use axum::Json;
use axum::body::Bytes;
use axum::extract::rejection::{BytesRejection, QueryRejection};
use axum::extract::{DefaultBodyLimit, Path, Query, State};
use axum::http::{StatusCode, header};
use axum::response::{IntoResponse, Response};
use oxistream_db::FeedKind as DbFeedKind;
use oxistream_db::{FeedCursor, FeedFilter, FeedVideo, Image, Subscription};
use rustypipe::validate;
use serde::{Deserialize, Serialize};
use time::OffsetDateTime;
use utoipa::{IntoParams, ToSchema};
use utoipa_axum::router::OpenApiRouter;
use utoipa_axum::routes;

use super::cursor;
use crate::AppState;
use crate::api::dto::{ChannelRef, Thumbnail, VideoSummary};
use crate::api::error::{ApiError, ErrorBody, ErrorCode};
use crate::auth::CurrentUser;
use crate::subscription_io::{self, ExportedChannel, ImportFormat};

const FEED_DEFAULT_LIMIT: u32 = 30;
const FEED_MAX_LIMIT: u32 = 100;
const IMPORT_MAX_BYTES: usize = 5 * 1024 * 1024;
const IMPORT_MAX_CHANNELS: usize = 10_000;
const IMPORT_MAX_SKIPPED_ENTRIES: usize = 100;

pub fn router() -> OpenApiRouter<AppState> {
    let import = OpenApiRouter::new()
        .routes(routes!(import_subscriptions))
        .layer(DefaultBodyLimit::max(IMPORT_MAX_BYTES));
    OpenApiRouter::new()
        .merge(import)
        .routes(routes!(export_subscriptions))
        .routes(routes!(list_subscriptions))
        .routes(routes!(subscription_state, subscribe, unsubscribe))
        .routes(routes!(visit_channel))
        .routes(routes!(feed))
}

/// A channel the signed-in user subscribed to.
#[derive(Debug, Serialize, ToSchema)]
pub struct SubscribedChannel {
    pub id: String,
    /// `null` until the channel was fetched once, shortly after the first subscription,
    /// unless an import provided it.
    pub name: Option<String>,
    pub handle: Option<String>,
    pub avatar: Vec<Thumbnail>,
    pub subscriber_count: Option<u64>,
    pub verified: bool,
    #[serde(with = "time::serde::rfc3339")]
    pub subscribed_at: OffsetDateTime,
    /// The channel published videos since the user last opened its page.
    pub has_new: bool,
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
            has_new: subscription.has_new,
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

fn check_channel_id(id: &str) -> Result<(), ApiError> {
    validate::channel_id(id).map_err(|_| ApiError::bad_request("invalid channel id"))
}

/// List subscriptions
///
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

/// Get subscription state
///
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

/// Subscribe to channel
///
/// Idempotent; new channels are fetched in the background.
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

/// Unsubscribe from channel
///
/// Succeeds if not subscribed, too.
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

/// Record channel visit
///
/// Records that the signed-in user opened a channel's page, clearing its `has_new`. Succeeds if not
/// subscribed, too.
#[utoipa::path(
    put,
    path = "/subscriptions/{channel_id}/visit",
    tag = "subscriptions",
    security(("session" = [])),
    params(("channel_id" = String, Path, description = "YouTube channel ID (`UC…`)")),
    responses(
        (status = NO_CONTENT, description = "Visit recorded"),
        (status = BAD_REQUEST, description = "Invalid channel ID", body = ErrorBody),
        (status = UNAUTHORIZED, description = "Not signed in", body = ErrorBody),
        (status = FORBIDDEN, description = "Foreign origin", body = ErrorBody),
    )
)]
async fn visit_channel(
    State(state): State<AppState>,
    CurrentUser(user): CurrentUser,
    Path(channel_id): Path<String>,
) -> Result<StatusCode, ApiError> {
    check_channel_id(&channel_id)?;
    state.subscriptions.visit(user.id, &channel_id).await?;
    Ok(StatusCode::NO_CONTENT)
}

/// Outcome of a subscription import.
#[derive(Debug, Serialize, ToSchema)]
pub struct ImportResult {
    /// Detected file format.
    pub format: ImportFormat,
    /// New subscriptions.
    pub imported: u64,
    /// Channels in the file the user was already subscribed to.
    pub already_subscribed: u64,
    /// Entries that cannot be imported, such as handles, custom URLs or other services.
    pub skipped: u64,
    /// Entries that are not channels.
    pub invalid: u64,
    /// The first skipped entries, as given in the file.
    pub skipped_entries: Vec<String>,
}

/// A subscription file as exported by another app.
#[derive(ToSchema)]
#[schema(value_type = String, format = Binary)]
#[allow(dead_code)] // Only describes the request body.
pub struct ImportFile(Vec<u8>);

/// Import subscriptions
///
/// Accepts a YouTube Takeout `subscriptions.csv`, a NewPipe, Invidious or Piped JSON export, or an
/// OPML file; the format is detected from the content. Idempotent. Only channels given by ID or
/// `/channel/UC…` URL can be imported. New channels are fetched by the feed worker over the
/// following minutes.
#[utoipa::path(
    post,
    path = "/subscriptions/import",
    tag = "subscriptions",
    security(("session" = [])),
    request_body(
        description = "The exported file as is, at most 5 MiB",
        content = ImportFile,
        content_type = "application/octet-stream",
    ),
    responses(
        (status = OK, body = ImportResult),
        (status = BAD_REQUEST, description = "Unrecognized or malformed file", body = ErrorBody),
        (status = UNAUTHORIZED, description = "Not signed in", body = ErrorBody),
        (status = FORBIDDEN, description = "Foreign origin", body = ErrorBody),
        (status = PAYLOAD_TOO_LARGE, description = "File too large", body = ErrorBody),
    )
)]
async fn import_subscriptions(
    State(state): State<AppState>,
    CurrentUser(user): CurrentUser,
    body: Result<Bytes, BytesRejection>,
) -> Result<Json<ImportResult>, ApiError> {
    let body = body.map_err(|rejection| {
        if rejection.status() == StatusCode::PAYLOAD_TOO_LARGE {
            ApiError::new(
                StatusCode::PAYLOAD_TOO_LARGE,
                ErrorCode::BadRequest,
                "file must not exceed 5 MiB",
            )
        } else {
            ApiError::bad_request(rejection.body_text())
        }
    })?;
    let file =
        subscription_io::parse(&body).map_err(|err| ApiError::bad_request(err.to_string()))?;
    if file.channels.len() > IMPORT_MAX_CHANNELS {
        return Err(ApiError::bad_request(format!(
            "at most {IMPORT_MAX_CHANNELS} channels can be imported at once"
        )));
    }
    // The feed worker picks up never-checked channels on its next tick, one at a time.
    let imported = if file.channels.is_empty() {
        0
    } else {
        state.subscriptions.import(user.id, &file.channels).await?
    };
    tracing::info!(
        user_id = %user.id,
        format = ?file.format,
        channels = file.channels.len(),
        imported,
        "imported subscriptions"
    );
    Ok(Json(ImportResult {
        format: file.format,
        imported,
        already_subscribed: file.channels.len() as u64 - imported,
        skipped: file.skipped.len() as u64,
        invalid: file.invalid as u64,
        skipped_entries: file
            .skipped
            .into_iter()
            .take(IMPORT_MAX_SKIPPED_ENTRIES)
            .collect(),
    }))
}

#[derive(Debug, Clone, Copy, Deserialize, ToSchema)]
#[serde(rename_all = "lowercase")]
pub enum ExportFormat {
    /// NewPipe JSON, also understood by Piped, LibreTube and Invidious.
    NewPipe,
    /// Invidious JSON.
    Invidious,
}

#[derive(Debug, Deserialize, IntoParams)]
#[into_params(parameter_in = Query)]
struct ExportParams {
    #[param(inline)]
    format: ExportFormat,
}

/// Export subscriptions
///
/// Downloads the signed-in user's subscriptions as a file.
#[utoipa::path(
    get,
    path = "/subscriptions/export",
    tag = "subscriptions",
    security(("session" = [])),
    params(ExportParams),
    responses(
        (status = OK, description = "The export, as an attachment", content_type = "application/json", body = Object),
        (status = BAD_REQUEST, description = "Unknown format", body = ErrorBody),
        (status = UNAUTHORIZED, description = "Not signed in", body = ErrorBody),
    )
)]
async fn export_subscriptions(
    State(state): State<AppState>,
    CurrentUser(user): CurrentUser,
    params: Result<Query<ExportParams>, QueryRejection>,
) -> Result<Response, ApiError> {
    let Query(params) =
        params.map_err(|_| ApiError::bad_request("`format` must be `newpipe` or `invidious`"))?;
    let channels: Vec<ExportedChannel> = state
        .subscriptions
        .list(user.id)
        .await?
        .into_iter()
        .map(|s| ExportedChannel {
            id: s.channel.id,
            name: s.channel.name,
        })
        .collect();
    let (prefix, body) = match params.format {
        ExportFormat::NewPipe => ("newpipe", subscription_io::to_newpipe(&channels)),
        ExportFormat::Invidious => ("invidious", subscription_io::to_invidious(&channels)),
    };
    let stamp = OffsetDateTime::now_utc()
        .format(time::macros::format_description!(
            "[year][month][day][hour][minute]"
        ))
        .map_err(|_| ApiError::internal())?;
    let disposition = format!("attachment; filename=\"{prefix}_subscriptions_{stamp}.json\"");
    Ok((
        [
            (header::CONTENT_TYPE, "application/json".to_owned()),
            (header::CONTENT_DISPOSITION, disposition),
            (header::CACHE_CONTROL, "no-store".to_owned()),
        ],
        body,
    )
        .into_response())
}

#[derive(Debug, Deserialize, IntoParams)]
#[into_params(parameter_in = Query)]
struct FeedParams {
    /// Token from a previous page.
    continuation: Option<String>,
    /// Page size, 1 to 100; default 30.
    limit: Option<u32>,
    /// Defaults to `all`.
    #[param(inline)]
    kind: Option<FeedKind>,
    /// Leave out finished videos.
    #[serde(default)]
    hide_watched: bool,
    /// Only videos of this subscribed channel (`UC…`).
    channel: Option<String>,
    /// Only videos of channels in this subscription group.
    group: Option<String>,
}

#[derive(Debug, Clone, Copy, Default, Deserialize, ToSchema)]
#[serde(rename_all = "lowercase")]
pub enum FeedKind {
    #[default]
    All,
    /// Everything but shorts.
    Videos,
    Shorts,
}

impl From<FeedKind> for DbFeedKind {
    fn from(kind: FeedKind) -> Self {
        match kind {
            FeedKind::All => Self::All,
            FeedKind::Videos => Self::Videos,
            FeedKind::Shorts => Self::Shorts,
        }
    }
}

/// Get subscription feed
///
/// Videos of subscribed channels, newest first. Videos the user is not interested in are left out.
#[utoipa::path(
    get,
    path = "/feed",
    tag = "subscriptions",
    security(("session" = [])),
    params(FeedParams),
    responses(
        (status = OK, body = FeedPage),
        (status = BAD_REQUEST, description = "Invalid continuation, limit, kind or channel", body = ErrorBody),
        (status = UNAUTHORIZED, description = "Not signed in", body = ErrorBody),
        (status = NOT_FOUND, description = "No such group of the user", body = ErrorBody),
    )
)]
async fn feed(
    State(state): State<AppState>,
    CurrentUser(user): CurrentUser,
    params: Result<Query<FeedParams>, QueryRejection>,
) -> Result<Json<FeedPage>, ApiError> {
    let Query(params) = params.map_err(|_| {
        ApiError::bad_request("invalid query; `kind` must be `all`, `videos` or `shorts`")
    })?;
    let channel_id = params
        .channel
        .map(|id| id.trim().to_owned())
        .filter(|id| !id.is_empty());
    if let Some(id) = &channel_id {
        check_channel_id(id)?;
    }
    let group_id = match params.group.as_deref().map(str::trim) {
        None | Some("") => None,
        Some(id) => {
            let id = super::groups::parse_group_id(id)?;
            // Foreign groups must not reveal which channels they hold.
            if state.groups.get(user.id, id).await?.is_none() {
                return Err(ApiError::new(
                    StatusCode::NOT_FOUND,
                    ErrorCode::NotFound,
                    "group not found",
                ));
            }
            Some(id)
        }
    };
    let filter = FeedFilter {
        kind: params.kind.unwrap_or_default().into(),
        hide_watched: params.hide_watched,
        channel_id,
        group_id,
    };
    let limit = params.limit.unwrap_or(FEED_DEFAULT_LIMIT);
    if !(1..=FEED_MAX_LIMIT).contains(&limit) {
        return Err(ApiError::bad_request(format!(
            "`limit` must be between 1 and {FEED_MAX_LIMIT}"
        )));
    }
    let after = match params.continuation.as_deref().map(str::trim) {
        None | Some("") => None,
        Some(token) => {
            let (sort_at, video_id) = cursor::decode(token)
                .ok_or_else(|| ApiError::bad_request("invalid continuation"))?;
            Some(FeedCursor { sort_at, video_id })
        }
    };
    let mut videos = state
        .subscriptions
        .feed(user.id, &filter, after.as_ref(), limit + 1)
        .await?;
    let continuation = if videos.len() > limit as usize {
        videos.truncate(limit as usize);
        videos
            .last()
            .map(|v| cursor::encode(v.sort_at, &v.video_id))
    } else {
        None
    };
    let img = |url: &str| state.image_url(url);
    Ok(Json(FeedPage {
        items: videos.into_iter().map(|v| feed_item(v, &img)).collect(),
        continuation,
    }))
}
