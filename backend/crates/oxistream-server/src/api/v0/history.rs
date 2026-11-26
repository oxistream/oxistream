use axum::Json;
use axum::body::Bytes;
use axum::extract::rejection::JsonRejection;
use axum::extract::{Path, Query, State};
use axum::http::StatusCode;
use oxistream_db::{HistoryCursor, HistoryEntry, ProgressUpdate, WatchProgress};
use rustypipe::validate;
use serde::{Deserialize, Serialize};
use time::OffsetDateTime;
use utoipa::{IntoParams, ToSchema};
use utoipa_axum::router::OpenApiRouter;
use utoipa_axum::routes;

use super::cursor;
use crate::AppState;
use crate::api::dto::{ChannelRef, Thumbnail, VideoSummary, default_thumbnails};
use crate::api::error::{ApiError, ErrorBody};
use crate::auth::CurrentUser;

const HISTORY_DEFAULT_LIMIT: u32 = 30;
const HISTORY_MAX_LIMIT: u32 = 100;
const CONTINUE_DEFAULT_LIMIT: u32 = 12;
const CONTINUE_MAX_LIMIT: u32 = 50;
const MAX_BATCH: usize = 100;
/// Videos left within the first seconds are not worth continuing.
const CONTINUE_MIN_POSITION: f64 = 10.0;
/// A week; longer durations are bogus reports.
const MAX_DURATION: f64 = 7.0 * 24.0 * 3600.0;

pub fn router() -> OpenApiRouter<AppState> {
    OpenApiRouter::new()
        .routes(routes!(report_progress, batch_progress))
        .routes(routes!(video_progress))
        .routes(routes!(history, clear_history))
        .routes(routes!(delete_entry))
        .routes(routes!(continue_watching))
        .routes(routes!(history_settings, update_history_settings))
}

/// A progress report from the player.
#[derive(Debug, Deserialize, ToSchema)]
pub struct ProgressReport {
    pub video_id: String,
    /// Seconds into the video.
    pub position_seconds: f64,
    pub duration_seconds: f64,
    /// Watched to (nearly) the end.
    #[serde(default)]
    pub finished: bool,
}

/// How far the signed-in user got into a video.
#[derive(Debug, Serialize, ToSchema)]
pub struct VideoProgress {
    pub video_id: String,
    pub position_seconds: f64,
    pub duration_seconds: f64,
    pub finished: bool,
    /// When the video was last watched.
    #[serde(with = "time::serde::rfc3339")]
    pub updated_at: OffsetDateTime,
}

#[derive(Debug, Serialize, ToSchema)]
pub struct ProgressState {
    /// `null` if the video was never watched.
    pub progress: Option<VideoProgress>,
}

#[derive(Debug, Serialize, ToSchema)]
pub struct HistoryItem {
    /// From the metadata cache; `title` is the video ID and `channel` is absent while the
    /// metadata is unknown.
    pub video: VideoSummary,
    pub progress: VideoProgress,
}

#[derive(Debug, Serialize, ToSchema)]
pub struct HistoryPage {
    /// Most recently watched first.
    pub items: Vec<HistoryItem>,
    /// Pass to `continuation` to fetch the next page; absent on the last page.
    pub continuation: Option<String>,
}

#[derive(Debug, Serialize, Deserialize, ToSchema)]
pub struct HistorySettings {
    /// Progress reports are ignored while paused; existing history stays.
    pub paused: bool,
}

impl From<WatchProgress> for VideoProgress {
    fn from(p: WatchProgress) -> Self {
        Self {
            video_id: p.video_id,
            position_seconds: p.position_seconds,
            duration_seconds: p.duration_seconds,
            finished: p.finished,
            updated_at: p.updated_at,
        }
    }
}

fn history_item(entry: HistoryEntry, img: &impl Fn(&str) -> String) -> HistoryItem {
    let id = entry.progress.video_id.clone();
    let channel = entry.channel_id.map(|channel_id| ChannelRef {
        avatar: entry
            .channel_avatar
            .iter()
            .map(|i| Thumbnail {
                url: img(&i.url),
                width: i.width,
                height: i.height,
            })
            .collect(),
        name: entry.channel_name.unwrap_or_else(|| channel_id.clone()),
        id: channel_id,
        subscriber_count: None,
        verified: entry.channel_verified,
    });
    // Rounded and clamped, so the cast cannot truncate.
    #[allow(clippy::cast_possible_truncation, clippy::cast_sign_loss)]
    let duration = entry
        .progress
        .duration_seconds
        .round()
        .clamp(0.0, MAX_DURATION) as u32;
    HistoryItem {
        video: VideoSummary {
            thumbnails: default_thumbnails(&id, img),
            title: entry.title.unwrap_or_else(|| id.clone()),
            duration_seconds: Some(duration),
            channel,
            published_at: entry.published_at,
            published_text: None,
            view_count: None,
            viewers: None,
            is_live: false,
            is_short: false,
            is_upcoming: false,
            description: None,
            id,
        },
        progress: entry.progress.into(),
    }
}

fn check_video_id(id: &str) -> Result<(), ApiError> {
    validate::video_id(id).map_err(|_| ApiError::bad_request("invalid video id"))
}

fn check_limit(limit: Option<u32>, default: u32, max: u32) -> Result<u32, ApiError> {
    let limit = limit.unwrap_or(default);
    if (1..=max).contains(&limit) {
        Ok(limit)
    } else {
        Err(ApiError::bad_request(format!(
            "`limit` must be between 1 and {max}"
        )))
    }
}

/// Record how far the signed-in user got into a video. Ignored while history is paused.
///
/// The body is JSON, sent as `application/json` or, for `navigator.sendBeacon`, `text/plain`.
#[utoipa::path(
    post,
    path = "/progress",
    tag = "history",
    security(("session" = [])),
    request_body(content = ProgressReport, content_type = "application/json"),
    responses(
        (status = NO_CONTENT, description = "Recorded, or ignored while history is paused"),
        (status = BAD_REQUEST, description = "Invalid report", body = ErrorBody),
        (status = UNAUTHORIZED, description = "Not signed in", body = ErrorBody),
        (status = FORBIDDEN, description = "Foreign origin", body = ErrorBody),
    )
)]
async fn report_progress(
    State(state): State<AppState>,
    CurrentUser(user): CurrentUser,
    body: Bytes,
) -> Result<StatusCode, ApiError> {
    let report: ProgressReport = serde_json::from_slice(&body)
        .map_err(|err| ApiError::bad_request(format!("invalid progress report: {err}")))?;
    check_video_id(&report.video_id)?;
    let duration = report.duration_seconds;
    if !duration.is_finite() || duration <= 0.0 || duration > MAX_DURATION {
        return Err(ApiError::bad_request("invalid duration"));
    }
    if !report.position_seconds.is_finite() || report.position_seconds < 0.0 {
        return Err(ApiError::bad_request("invalid position"));
    }
    state
        .history
        .record(
            user.id,
            &ProgressUpdate {
                video_id: report.video_id,
                position_seconds: report.position_seconds.min(duration),
                duration_seconds: duration,
                finished: report.finished,
            },
        )
        .await?;
    Ok(StatusCode::NO_CONTENT)
}

#[derive(Debug, Deserialize, IntoParams)]
#[into_params(parameter_in = Query)]
struct BatchParams {
    /// Comma-separated video IDs, at most 100.
    ids: String,
}

/// Progress of several videos, e.g. for the progress bars of a list. Videos never watched
/// are left out.
#[utoipa::path(
    get,
    path = "/progress",
    tag = "history",
    security(("session" = [])),
    params(BatchParams),
    responses(
        (status = OK, body = Vec<VideoProgress>),
        (status = BAD_REQUEST, description = "Invalid or too many IDs", body = ErrorBody),
        (status = UNAUTHORIZED, description = "Not signed in", body = ErrorBody),
    )
)]
async fn batch_progress(
    State(state): State<AppState>,
    CurrentUser(user): CurrentUser,
    Query(params): Query<BatchParams>,
) -> Result<Json<Vec<VideoProgress>>, ApiError> {
    let mut ids: Vec<String> = params
        .ids
        .split(',')
        .map(str::trim)
        .filter(|id| !id.is_empty())
        .map(str::to_owned)
        .collect();
    ids.sort_unstable();
    ids.dedup();
    if ids.len() > MAX_BATCH {
        return Err(ApiError::bad_request(format!(
            "at most {MAX_BATCH} IDs per request"
        )));
    }
    for id in &ids {
        check_video_id(id)?;
    }
    let progress = state.history.progress(user.id, &ids).await?;
    Ok(Json(progress.into_iter().map(Into::into).collect()))
}

/// Progress of one video, to resume where the signed-in user left off.
#[utoipa::path(
    get,
    path = "/progress/{video_id}",
    tag = "history",
    security(("session" = [])),
    params(("video_id" = String, Path, description = "YouTube video ID")),
    responses(
        (status = OK, body = ProgressState),
        (status = BAD_REQUEST, description = "Invalid video ID", body = ErrorBody),
        (status = UNAUTHORIZED, description = "Not signed in", body = ErrorBody),
    )
)]
async fn video_progress(
    State(state): State<AppState>,
    CurrentUser(user): CurrentUser,
    Path(video_id): Path<String>,
) -> Result<Json<ProgressState>, ApiError> {
    check_video_id(&video_id)?;
    let progress = state
        .history
        .progress(user.id, std::slice::from_ref(&video_id))
        .await?
        .into_iter()
        .next()
        .map(Into::into);
    Ok(Json(ProgressState { progress }))
}

#[derive(Debug, Deserialize, IntoParams)]
#[into_params(parameter_in = Query)]
struct HistoryParams {
    /// Token from a previous page.
    continuation: Option<String>,
    /// Page size, 1 to 100; default 30.
    limit: Option<u32>,
}

/// Watched videos, most recently watched first.
#[utoipa::path(
    get,
    path = "/history",
    tag = "history",
    security(("session" = [])),
    params(HistoryParams),
    responses(
        (status = OK, body = HistoryPage),
        (status = BAD_REQUEST, description = "Invalid continuation or limit", body = ErrorBody),
        (status = UNAUTHORIZED, description = "Not signed in", body = ErrorBody),
    )
)]
async fn history(
    State(state): State<AppState>,
    CurrentUser(user): CurrentUser,
    Query(params): Query<HistoryParams>,
) -> Result<Json<HistoryPage>, ApiError> {
    let limit = check_limit(params.limit, HISTORY_DEFAULT_LIMIT, HISTORY_MAX_LIMIT)?;
    let after = match params.continuation.as_deref().map(str::trim) {
        None | Some("") => None,
        Some(token) => {
            let (updated_at, video_id) = cursor::decode(token)
                .ok_or_else(|| ApiError::bad_request("invalid continuation"))?;
            Some(HistoryCursor {
                updated_at,
                video_id,
            })
        }
    };
    let mut entries = state
        .history
        .list(user.id, after.as_ref(), limit + 1)
        .await?;
    let continuation = if entries.len() > limit as usize {
        entries.truncate(limit as usize);
        entries.last().map(|e| {
            let c = e.cursor();
            cursor::encode(c.updated_at, &c.video_id)
        })
    } else {
        None
    };
    let img = |url: &str| state.image_url(url);
    Ok(Json(HistoryPage {
        items: entries.into_iter().map(|e| history_item(e, &img)).collect(),
        continuation,
    }))
}

/// Clear the whole watch history, including resume points.
#[utoipa::path(
    delete,
    path = "/history",
    tag = "history",
    security(("session" = [])),
    responses(
        (status = NO_CONTENT, description = "History cleared"),
        (status = UNAUTHORIZED, description = "Not signed in", body = ErrorBody),
        (status = FORBIDDEN, description = "Foreign origin", body = ErrorBody),
    )
)]
async fn clear_history(
    State(state): State<AppState>,
    CurrentUser(user): CurrentUser,
) -> Result<StatusCode, ApiError> {
    state.history.clear(user.id).await?;
    Ok(StatusCode::NO_CONTENT)
}

/// Remove one video from the watch history. Succeeds if it is not in there, too.
#[utoipa::path(
    delete,
    path = "/history/{video_id}",
    tag = "history",
    security(("session" = [])),
    params(("video_id" = String, Path, description = "YouTube video ID")),
    responses(
        (status = NO_CONTENT, description = "Not in the history anymore"),
        (status = BAD_REQUEST, description = "Invalid video ID", body = ErrorBody),
        (status = UNAUTHORIZED, description = "Not signed in", body = ErrorBody),
        (status = FORBIDDEN, description = "Foreign origin", body = ErrorBody),
    )
)]
async fn delete_entry(
    State(state): State<AppState>,
    CurrentUser(user): CurrentUser,
    Path(video_id): Path<String>,
) -> Result<StatusCode, ApiError> {
    check_video_id(&video_id)?;
    state.history.delete(user.id, &video_id).await?;
    Ok(StatusCode::NO_CONTENT)
}

#[derive(Debug, Deserialize, IntoParams)]
#[into_params(parameter_in = Query)]
struct ContinueParams {
    /// 1 to 50; default 12.
    limit: Option<u32>,
}

/// Started but unfinished videos, most recently watched first.
#[utoipa::path(
    get,
    path = "/history/continue",
    tag = "history",
    security(("session" = [])),
    params(ContinueParams),
    responses(
        (status = OK, body = Vec<HistoryItem>),
        (status = BAD_REQUEST, description = "Invalid limit", body = ErrorBody),
        (status = UNAUTHORIZED, description = "Not signed in", body = ErrorBody),
    )
)]
async fn continue_watching(
    State(state): State<AppState>,
    CurrentUser(user): CurrentUser,
    Query(params): Query<ContinueParams>,
) -> Result<Json<Vec<HistoryItem>>, ApiError> {
    let limit = check_limit(params.limit, CONTINUE_DEFAULT_LIMIT, CONTINUE_MAX_LIMIT)?;
    let entries = state
        .history
        .unfinished(user.id, CONTINUE_MIN_POSITION, limit)
        .await?;
    let img = |url: &str| state.image_url(url);
    Ok(Json(
        entries.into_iter().map(|e| history_item(e, &img)).collect(),
    ))
}

/// Whether the watch history is paused.
#[utoipa::path(
    get,
    path = "/history/settings",
    tag = "history",
    security(("session" = [])),
    responses(
        (status = OK, body = HistorySettings),
        (status = UNAUTHORIZED, description = "Not signed in", body = ErrorBody),
    )
)]
async fn history_settings(
    State(state): State<AppState>,
    CurrentUser(user): CurrentUser,
) -> Result<Json<HistorySettings>, ApiError> {
    let settings = state.settings.get(user.id).await?;
    Ok(Json(HistorySettings {
        paused: settings.history_paused,
    }))
}

/// Pause or resume the watch history.
#[utoipa::path(
    put,
    path = "/history/settings",
    tag = "history",
    security(("session" = [])),
    request_body = HistorySettings,
    responses(
        (status = OK, body = HistorySettings),
        (status = BAD_REQUEST, description = "Invalid body", body = ErrorBody),
        (status = UNAUTHORIZED, description = "Not signed in", body = ErrorBody),
        (status = FORBIDDEN, description = "Foreign origin", body = ErrorBody),
    )
)]
async fn update_history_settings(
    State(state): State<AppState>,
    CurrentUser(user): CurrentUser,
    body: Result<Json<HistorySettings>, JsonRejection>,
) -> Result<Json<HistorySettings>, ApiError> {
    let Json(body) = body?;
    let settings = state
        .settings
        .set_history_paused(user.id, body.paused)
        .await?;
    Ok(Json(HistorySettings {
        paused: settings.history_paused,
    }))
}
