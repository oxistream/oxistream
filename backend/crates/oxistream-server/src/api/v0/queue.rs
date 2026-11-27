use std::collections::HashSet;

use axum::Json;
use axum::extract::rejection::JsonRejection;
use axum::extract::{Path, State};
use oxistream_db::{QueueOp, QueuePosition};
use rustypipe::validate;
use serde::{Deserialize, Serialize};
use utoipa::ToSchema;
use utoipa_axum::router::OpenApiRouter;
use utoipa_axum::routes;

use super::playlists::{check_video_id, video_thumbnails};
use crate::AppState;
use crate::api::dto::{ChannelRef, VideoSummary};
use crate::api::error::{ApiError, ErrorBody};
use crate::auth::CurrentUser;

/// Longer queues lose entries at the end.
pub const MAX_ENTRIES: usize = 200;
const MAX_KEY_CHARS: usize = 64;
const MAX_TITLE_CHARS: usize = 500;
const MAX_CHANNEL_NAME_CHARS: usize = 200;

pub fn router() -> OpenApiRouter<AppState> {
    OpenApiRouter::new()
        .routes(routes!(get_queue, clear_queue))
        .routes(routes!(add_entries))
        .routes(routes!(remove_entry))
        .routes(routes!(move_entry))
}

/// The signed-in user's up-next queue, in play order.
#[derive(Debug, Serialize, ToSchema)]
pub struct UpNextQueue {
    /// Grows with every change, so clients can tell newer states from older ones.
    pub revision: i64,
    pub entries: Vec<UpNextEntry>,
}

#[derive(Debug, Serialize, ToSchema)]
pub struct UpNextEntry {
    pub key: String,
    /// As sent by the client; view counts and dates are never set.
    pub video: VideoSummary,
}

#[derive(Debug, Deserialize, ToSchema)]
pub struct NewUpNextEntry {
    /// Chosen by the client and unique within the queue: 1 to 64 of `A-Z a-z 0-9 _ -`.
    pub key: String,
    pub video_id: String,
    /// 1 to 500 characters.
    pub title: String,
    pub channel_id: Option<String>,
    /// Up to 200 characters; ignored without `channel_id`.
    pub channel_name: Option<String>,
    pub duration_seconds: Option<u32>,
}

#[derive(Debug, Clone, Copy, Default, Deserialize, ToSchema)]
#[serde(rename_all = "snake_case")]
pub enum UpNextPosition {
    /// Plays next.
    Start,
    #[default]
    End,
}

#[derive(Debug, Deserialize, ToSchema)]
pub struct AddToUpNext {
    /// Up to 200. Entries whose key is already queued are left out, so retries are harmless.
    pub entries: Vec<NewUpNextEntry>,
    /// Default `end`.
    #[serde(default)]
    pub position: UpNextPosition,
    /// Also leaves out videos that are already queued, e.g. to merge a queue from before
    /// signing in.
    #[serde(default)]
    pub skip_queued: bool,
}

#[derive(Debug, Deserialize, ToSchema)]
pub struct MoveUpNextEntry {
    /// Zero-based target position; past the end moves the entry to the end.
    pub index: u32,
}

fn queue_dto(queue: oxistream_db::Queue, img: &impl Fn(&str) -> String) -> UpNextQueue {
    UpNextQueue {
        revision: queue.revision,
        entries: queue
            .entries
            .into_iter()
            .map(|e| UpNextEntry {
                video: VideoSummary {
                    thumbnails: video_thumbnails(&e.video_id, img),
                    channel: e.channel_id.map(|id| ChannelRef {
                        id,
                        name: e.channel_name.unwrap_or_default(),
                        avatar: Vec::new(),
                        subscriber_count: None,
                        verified: false,
                    }),
                    duration_seconds: e.duration_seconds.and_then(|d| u32::try_from(d).ok()),
                    published_at: None,
                    published_text: None,
                    view_count: None,
                    viewers: None,
                    is_live: false,
                    is_short: false,
                    is_upcoming: false,
                    description: None,
                    id: e.video_id,
                    title: e.title,
                },
                key: e.key,
            })
            .collect(),
    }
}

fn check_key(key: &str) -> Result<(), ApiError> {
    let valid = (1..=MAX_KEY_CHARS).contains(&key.len())
        && key
            .bytes()
            .all(|b| b.is_ascii_alphanumeric() || b == b'-' || b == b'_');
    if valid {
        Ok(())
    } else {
        Err(ApiError::bad_request("invalid queue entry key"))
    }
}

fn entry_record(entry: NewUpNextEntry) -> Result<oxistream_db::QueueEntry, ApiError> {
    check_key(&entry.key)?;
    check_video_id(&entry.video_id)?;
    let title = entry.title.trim();
    if title.is_empty() || title.chars().count() > MAX_TITLE_CHARS {
        return Err(ApiError::bad_request(format!(
            "`title` must have 1 to {MAX_TITLE_CHARS} characters"
        )));
    }
    let channel_id = entry.channel_id.filter(|id| !id.is_empty());
    if let Some(id) = &channel_id {
        validate::channel_id(id).map_err(|_| ApiError::bad_request("invalid channel id"))?;
    }
    let channel_name = channel_id
        .as_ref()
        .map(|_| entry.channel_name.unwrap_or_default().trim().to_owned());
    if channel_name
        .as_ref()
        .is_some_and(|n| n.chars().count() > MAX_CHANNEL_NAME_CHARS)
    {
        return Err(ApiError::bad_request(format!(
            "`channel_name` must have at most {MAX_CHANNEL_NAME_CHARS} characters"
        )));
    }
    Ok(oxistream_db::QueueEntry {
        key: entry.key,
        video_id: entry.video_id,
        title: title.to_owned(),
        channel_id,
        channel_name,
        duration_seconds: entry.duration_seconds.and_then(|d| i32::try_from(d).ok()),
    })
}

async fn apply(
    state: &AppState,
    user_id: oxistream_db::Uuid,
    op: QueueOp,
) -> Result<Json<UpNextQueue>, ApiError> {
    let queue = state.queue.apply(user_id, &op, MAX_ENTRIES).await?;
    Ok(Json(queue_dto(queue, &|url| state.image_url(url))))
}

/// The signed-in user's up-next queue. Guests keep theirs in the browser.
#[utoipa::path(
    get,
    path = "/queue",
    tag = "queue",
    security(("session" = [])),
    responses(
        (status = OK, body = UpNextQueue),
        (status = UNAUTHORIZED, description = "Not signed in", body = ErrorBody),
    )
)]
async fn get_queue(
    State(state): State<AppState>,
    CurrentUser(user): CurrentUser,
) -> Result<Json<UpNextQueue>, ApiError> {
    let queue = state.queue.get(user.id).await?;
    Ok(Json(queue_dto(queue, &|url| state.image_url(url))))
}

/// Remove all entries.
#[utoipa::path(
    delete,
    path = "/queue",
    tag = "queue",
    security(("session" = [])),
    responses(
        (status = OK, description = "The queue after the change", body = UpNextQueue),
        (status = UNAUTHORIZED, description = "Not signed in", body = ErrorBody),
        (status = FORBIDDEN, description = "Foreign origin", body = ErrorBody),
    )
)]
async fn clear_queue(
    State(state): State<AppState>,
    CurrentUser(user): CurrentUser,
) -> Result<Json<UpNextQueue>, ApiError> {
    apply(&state, user.id, QueueOp::Clear).await
}

/// Add entries at the start or end. A full queue loses entries at the end.
#[utoipa::path(
    post,
    path = "/queue/entries",
    tag = "queue",
    security(("session" = [])),
    request_body = AddToUpNext,
    responses(
        (status = OK, description = "The queue after the change", body = UpNextQueue),
        (status = BAD_REQUEST, description = "Invalid entry", body = ErrorBody),
        (status = UNAUTHORIZED, description = "Not signed in", body = ErrorBody),
        (status = FORBIDDEN, description = "Foreign origin", body = ErrorBody),
    )
)]
async fn add_entries(
    State(state): State<AppState>,
    CurrentUser(user): CurrentUser,
    body: Result<Json<AddToUpNext>, JsonRejection>,
) -> Result<Json<UpNextQueue>, ApiError> {
    let Json(body) = body?;
    if body.entries.len() > MAX_ENTRIES {
        return Err(ApiError::bad_request(format!(
            "at most {MAX_ENTRIES} entries at once"
        )));
    }
    let entries = body
        .entries
        .into_iter()
        .map(entry_record)
        .collect::<Result<Vec<_>, _>>()?;
    let mut keys = HashSet::new();
    if !entries.iter().all(|e| keys.insert(e.key.as_str())) {
        return Err(ApiError::bad_request("duplicate queue entry key"));
    }
    let position = match body.position {
        UpNextPosition::Start => QueuePosition::Start,
        UpNextPosition::End => QueuePosition::End,
    };
    let op = QueueOp::Add {
        entries,
        position,
        skip_queued: body.skip_queued,
    };
    apply(&state, user.id, op).await
}

/// Remove an entry. Succeeds if it is not queued, too, so two players taking the same
/// entry do not fail.
#[utoipa::path(
    delete,
    path = "/queue/entries/{key}",
    tag = "queue",
    security(("session" = [])),
    params(("key" = String, Path, description = "Entry key")),
    responses(
        (status = OK, description = "The queue after the change", body = UpNextQueue),
        (status = BAD_REQUEST, description = "Invalid key", body = ErrorBody),
        (status = UNAUTHORIZED, description = "Not signed in", body = ErrorBody),
        (status = FORBIDDEN, description = "Foreign origin", body = ErrorBody),
    )
)]
async fn remove_entry(
    State(state): State<AppState>,
    CurrentUser(user): CurrentUser,
    Path(key): Path<String>,
) -> Result<Json<UpNextQueue>, ApiError> {
    check_key(&key)?;
    apply(&state, user.id, QueueOp::Remove { key }).await
}

/// Move an entry to another position. Succeeds if it is not queued, too.
#[utoipa::path(
    put,
    path = "/queue/entries/{key}/position",
    tag = "queue",
    security(("session" = [])),
    params(("key" = String, Path, description = "Entry key")),
    request_body = MoveUpNextEntry,
    responses(
        (status = OK, description = "The queue after the change", body = UpNextQueue),
        (status = BAD_REQUEST, description = "Invalid key", body = ErrorBody),
        (status = UNAUTHORIZED, description = "Not signed in", body = ErrorBody),
        (status = FORBIDDEN, description = "Foreign origin", body = ErrorBody),
    )
)]
async fn move_entry(
    State(state): State<AppState>,
    CurrentUser(user): CurrentUser,
    Path(key): Path<String>,
    body: Result<Json<MoveUpNextEntry>, JsonRejection>,
) -> Result<Json<UpNextQueue>, ApiError> {
    check_key(&key)?;
    let Json(body) = body?;
    apply(
        &state,
        user.id,
        QueueOp::Move {
            key,
            index: body.index,
        },
    )
    .await
}
