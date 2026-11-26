use std::collections::BTreeMap;

use axum::Json;
use axum::extract::rejection::JsonRejection;
use axum::extract::{Path, Query, State};
use axum::http::StatusCode;
use axum::response::{IntoResponse, Response};
use base64::Engine;
use base64::engine::general_purpose::URL_SAFE_NO_PAD;
use oxistream_db::{NewPlaylist, PlaylistItem, PlaylistUpdate, Video, VideoUpsert, Visibility};
use rustypipe::model::richtext::ToPlaintext;
use rustypipe::validate;
use serde::{Deserialize, Serialize};
use time::OffsetDateTime;
use utoipa::{IntoParams, ToSchema};
use utoipa_axum::router::OpenApiRouter;
use utoipa_axum::routes;

use super::youtube::video_record;
use crate::AppState;
use crate::api::dto::{ChannelRef, Thumbnail, VideoSummary};
use crate::api::error::{ApiError, ErrorBody, ErrorCode, FieldError};
use crate::api::lang::ContentLanguage;
use crate::auth::{CurrentUser, OptionalUser};

pub const MAX_PLAYLISTS: u32 = 1000;
pub const MAX_ITEMS: u32 = 5000;
pub const MAX_TITLE_CHARS: usize = 150;
pub const MAX_DESCRIPTION_CHARS: usize = 5000;
const ID_BYTES: usize = 12;

pub fn router() -> OpenApiRouter<AppState> {
    OpenApiRouter::new()
        .routes(routes!(list_playlists, create_playlist))
        .routes(routes!(import_playlist))
        .routes(routes!(get_playlist, update_playlist, delete_playlist))
        .routes(routes!(add_item, remove_item))
        .routes(routes!(move_item))
        .routes(routes!(set_last_played))
}

/// A new random playlist ID: 16 URL-safe characters.
pub fn new_playlist_id() -> String {
    let mut bytes = [0u8; ID_BYTES];
    getrandom::fill(&mut bytes).expect("system random number generator");
    URL_SAFE_NO_PAD.encode(bytes)
}

pub fn is_playlist_id(id: &str) -> bool {
    id.len() == 16
        && id
            .bytes()
            .all(|b| b.is_ascii_alphanumeric() || b == b'-' || b == b'_')
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, ToSchema)]
#[serde(rename_all = "snake_case")]
pub enum PlaylistVisibility {
    /// Only the owner can see it.
    Private,
    /// Anyone with the link can see it, signed in or not.
    Unlisted,
}

impl From<Visibility> for PlaylistVisibility {
    fn from(v: Visibility) -> Self {
        match v {
            Visibility::Private => Self::Private,
            Visibility::Unlisted => Self::Unlisted,
        }
    }
}

impl From<PlaylistVisibility> for Visibility {
    fn from(v: PlaylistVisibility) -> Self {
        match v {
            PlaylistVisibility::Private => Self::Private,
            PlaylistVisibility::Unlisted => Self::Unlisted,
        }
    }
}

/// A local playlist.
#[derive(Debug, Serialize, ToSchema)]
pub struct Playlist {
    /// Random ID, used in `/playlist?list=`.
    pub id: String,
    pub title: String,
    pub description: String,
    pub visibility: PlaylistVisibility,
    pub item_count: u32,
    /// Of the first item; empty for an empty playlist.
    pub thumbnails: Vec<Thumbnail>,
    /// The item the owner last played, to continue from. Only shown to the owner.
    pub last_video_id: Option<String>,
    /// Whether the playlist contains the video asked for with `video_id`; absent otherwise.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub contains_video: Option<bool>,
    #[serde(with = "time::serde::rfc3339")]
    pub created_at: OffsetDateTime,
    #[serde(with = "time::serde::rfc3339")]
    pub updated_at: OffsetDateTime,
}

#[derive(Debug, Serialize, ToSchema)]
pub struct PlaylistEntry {
    /// `channel` is always set; view counts and dates are not.
    pub video: VideoSummary,
    #[serde(with = "time::serde::rfc3339")]
    pub added_at: OffsetDateTime,
}

/// A playlist with its items, in order.
#[derive(Debug, Serialize, ToSchema)]
pub struct PlaylistDetails {
    #[serde(flatten)]
    pub playlist: Playlist,
    /// Username of the owner.
    pub owner: String,
    /// Whether the signed-in user owns the playlist and so may change it.
    pub is_owner: bool,
    pub items: Vec<PlaylistEntry>,
}

#[derive(Debug, Deserialize, ToSchema)]
pub struct CreatePlaylist {
    /// 1 to 150 characters.
    pub title: String,
    /// Up to 5000 characters.
    #[serde(default)]
    pub description: Option<String>,
    /// Default `private`.
    #[serde(default)]
    pub visibility: Option<PlaylistVisibility>,
}

/// Fields to change; absent fields stay as they are.
#[derive(Debug, Deserialize, ToSchema)]
pub struct UpdatePlaylist {
    pub title: Option<String>,
    pub description: Option<String>,
    pub visibility: Option<PlaylistVisibility>,
}

#[derive(Debug, Deserialize, ToSchema)]
pub struct ImportPlaylist {
    /// YouTube playlist ID; mixes cannot be imported.
    pub playlist_id: String,
}

#[derive(Debug, Deserialize, ToSchema)]
pub struct MoveItem {
    /// Zero-based target position; past the end moves the item to the end.
    pub index: u32,
}

#[derive(Debug, Deserialize, ToSchema)]
pub struct LastPlayed {
    pub video_id: String,
}

/// YouTube's standard thumbnails, available for every video.
fn video_thumbnails(video_id: &str, img: &impl Fn(&str) -> String) -> Vec<Thumbnail> {
    [("mqdefault", 320, 180), ("hqdefault", 480, 360)]
        .into_iter()
        .map(|(name, width, height)| Thumbnail {
            url: img(&format!("https://i.ytimg.com/vi/{video_id}/{name}.jpg")),
            width,
            height,
        })
        .collect()
}

fn playlist_dto(
    p: oxistream_db::Playlist,
    owner_view: bool,
    img: &impl Fn(&str) -> String,
) -> Playlist {
    Playlist {
        thumbnails: p
            .first_video_id
            .as_deref()
            .map(|id| video_thumbnails(id, img))
            .unwrap_or_default(),
        item_count: u32::try_from(p.item_count).unwrap_or(u32::MAX),
        visibility: p.visibility.into(),
        last_video_id: p.last_video_id.filter(|_| owner_view),
        contains_video: p.contains_video,
        created_at: p.created_at,
        updated_at: p.updated_at,
        id: p.id,
        title: p.title,
        description: p.description,
    }
}

/// A video from the metadata cache; `channel` is always set, view counts are not.
pub(super) fn cached_video_summary(v: Video, img: &impl Fn(&str) -> String) -> VideoSummary {
    VideoSummary {
        thumbnails: video_thumbnails(&v.id, img),
        channel: Some(ChannelRef {
            id: v.channel_id,
            name: v.channel_name,
            avatar: Vec::new(),
            subscriber_count: None,
            verified: false,
        }),
        duration_seconds: v.duration_seconds.and_then(|d| u32::try_from(d).ok()),
        published_at: v.published_at,
        published_text: None,
        view_count: None,
        viewers: None,
        is_live: false,
        is_short: false,
        is_upcoming: false,
        description: None,
        id: v.id,
        title: v.title,
    }
}

fn entry_dto(item: PlaylistItem, img: &impl Fn(&str) -> String) -> PlaylistEntry {
    PlaylistEntry {
        video: cached_video_summary(item.video, img),
        added_at: item.added_at,
    }
}

fn not_found() -> ApiError {
    ApiError::new(
        StatusCode::NOT_FOUND,
        ErrorCode::NotFound,
        "playlist not found",
    )
}

fn limit_reached(message: String) -> ApiError {
    ApiError::new(StatusCode::CONFLICT, ErrorCode::LimitReached, message)
}

/// Malformed IDs cannot exist, so they get the same 404 as unknown ones.
fn check_playlist_id(id: &str) -> Result<(), ApiError> {
    if is_playlist_id(id) {
        Ok(())
    } else {
        Err(not_found())
    }
}

pub(super) fn check_video_id(id: &str) -> Result<(), ApiError> {
    validate::video_id(id).map_err(|_| ApiError::bad_request("invalid video id"))
}

fn validate_title(title: &str, fields: &mut BTreeMap<String, FieldError>) -> String {
    let title = title.trim();
    if title.is_empty() {
        fields.insert("title".into(), FieldError::Required);
    } else if title.chars().count() > MAX_TITLE_CHARS {
        fields.insert("title".into(), FieldError::TooLong);
    }
    title.to_owned()
}

fn validate_description(description: &str, fields: &mut BTreeMap<String, FieldError>) -> String {
    let description = description.trim();
    if description.chars().count() > MAX_DESCRIPTION_CHARS {
        fields.insert("description".into(), FieldError::TooLong);
    }
    description.to_owned()
}

fn check_fields(fields: BTreeMap<String, FieldError>) -> Result<(), ApiError> {
    if fields.is_empty() {
        Ok(())
    } else {
        Err(ApiError::validation(
            StatusCode::UNPROCESSABLE_ENTITY,
            fields,
        ))
    }
}

fn map_limit(err: oxistream_db::Error, message: impl FnOnce() -> String) -> ApiError {
    match err {
        oxistream_db::Error::LimitReached => limit_reached(message()),
        err => err.into(),
    }
}

/// Caches the video's metadata unless already known, so it can be listed without YouTube.
pub(super) async fn ensure_cached(
    state: &AppState,
    video_id: &str,
    lang: rustypipe::param::Language,
) -> Result<(), ApiError> {
    if state.videos.get(video_id).await?.is_some() {
        return Ok(());
    }
    let (details, player) = tokio::join!(
        state.youtube.video_details(video_id, lang),
        state.youtube.player(video_id)
    );
    let details = details?;
    let player = player.ok();
    state
        .videos
        .upsert(&video_record(&details, player.as_deref()))
        .await?;
    Ok(())
}

#[derive(Debug, Deserialize, IntoParams)]
#[into_params(parameter_in = Query)]
struct ListParams {
    /// Report for each playlist whether it contains this video, in `contains_video`.
    video_id: Option<String>,
}

/// The signed-in user's playlists, most recently changed first.
#[utoipa::path(
    get,
    path = "/playlists",
    tag = "playlists",
    security(("session" = [])),
    params(ListParams),
    responses(
        (status = OK, body = Vec<Playlist>),
        (status = BAD_REQUEST, description = "Invalid video ID", body = ErrorBody),
        (status = UNAUTHORIZED, description = "Not signed in", body = ErrorBody),
    )
)]
async fn list_playlists(
    State(state): State<AppState>,
    CurrentUser(user): CurrentUser,
    Query(params): Query<ListParams>,
) -> Result<Json<Vec<Playlist>>, ApiError> {
    if let Some(id) = &params.video_id {
        check_video_id(id)?;
    }
    let img = |url: &str| state.image_url(url);
    let list = state
        .playlists
        .list(user.id, params.video_id.as_deref())
        .await?;
    Ok(Json(
        list.into_iter()
            .map(|p| playlist_dto(p, true, &img))
            .collect(),
    ))
}

/// Create a playlist.
#[utoipa::path(
    post,
    path = "/playlists",
    tag = "playlists",
    security(("session" = [])),
    request_body = CreatePlaylist,
    responses(
        (status = CREATED, body = Playlist),
        (status = UNAUTHORIZED, description = "Not signed in", body = ErrorBody),
        (status = FORBIDDEN, description = "Foreign origin", body = ErrorBody),
        (status = CONFLICT, description = "Too many playlists", body = ErrorBody),
        (status = UNPROCESSABLE_ENTITY, description = "Invalid title or description", body = ErrorBody),
    )
)]
async fn create_playlist(
    State(state): State<AppState>,
    CurrentUser(user): CurrentUser,
    body: Result<Json<CreatePlaylist>, JsonRejection>,
) -> Result<Response, ApiError> {
    let Json(body) = body?;
    let mut fields = BTreeMap::new();
    let title = validate_title(&body.title, &mut fields);
    let description = validate_description(body.description.as_deref().unwrap_or(""), &mut fields);
    check_fields(fields)?;
    let new = NewPlaylist {
        id: new_playlist_id(),
        title,
        description,
        visibility: body
            .visibility
            .unwrap_or(PlaylistVisibility::Private)
            .into(),
    };
    let created = state
        .playlists
        .create(user.id, &new, MAX_PLAYLISTS)
        .await
        .map_err(|e| map_limit(e, || format!("at most {MAX_PLAYLISTS} playlists allowed")))?;
    let body = playlist_dto(created, true, &|url| state.image_url(url));
    Ok((StatusCode::CREATED, Json(body)).into_response())
}

fn truncate_chars(text: &str, max: usize) -> String {
    text.trim()
        .chars()
        .take(max)
        .collect::<String>()
        .trim_end()
        .to_owned()
}

fn imported_video(v: rustypipe::model::VideoItem) -> Option<VideoUpsert> {
    // Unavailable videos come without a channel.
    let channel = v.channel?;
    Some(VideoUpsert {
        thumbnail_url: v
            .thumbnail
            .iter()
            .max_by_key(|t| t.width * t.height)
            .map(|t| t.url.clone()),
        duration_seconds: v.duration.and_then(|d| i32::try_from(d).ok()),
        published_at: v.publish_date,
        channel_id: channel.id,
        channel_name: channel.name,
        id: v.id,
        title: v.name,
    })
}

/// Copy a YouTube playlist into a new private playlist, which can then be edited. Copies at
/// most 5000 videos; unavailable ones are skipped. Like any local playlist, the copy holds each
/// video once: a repeated video keeps its first place.
#[utoipa::path(
    post,
    path = "/playlists/import",
    tag = "playlists",
    security(("session" = [])),
    params(("Accept-Language" = Option<String>, Header, description = "Content language")),
    request_body = ImportPlaylist,
    responses(
        (status = CREATED, body = Playlist),
        (status = BAD_REQUEST, description = "Invalid playlist ID", body = ErrorBody),
        (status = UNAUTHORIZED, description = "Not signed in", body = ErrorBody),
        (status = FORBIDDEN, description = "Foreign origin", body = ErrorBody),
        (status = NOT_FOUND, description = "Not found, private, or a mix (`reason` is `mix`)", body = ErrorBody),
        (status = CONFLICT, description = "Too many playlists", body = ErrorBody),
        (status = BAD_GATEWAY, description = "YouTube request failed", body = ErrorBody),
        (status = SERVICE_UNAVAILABLE, description = "Rate limited by YouTube", body = ErrorBody),
    )
)]
async fn import_playlist(
    State(state): State<AppState>,
    CurrentUser(user): CurrentUser,
    ContentLanguage(lang): ContentLanguage,
    body: Result<Json<ImportPlaylist>, JsonRejection>,
) -> Result<Response, ApiError> {
    let Json(body) = body?;
    let (playlist, videos) = state
        .youtube
        .playlist_with_videos(&body.playlist_id, MAX_ITEMS as usize, lang)
        .await?;
    let title = truncate_chars(&playlist.name, MAX_TITLE_CHARS);
    let new = NewPlaylist {
        id: new_playlist_id(),
        title: if title.is_empty() {
            body.playlist_id.clone()
        } else {
            title
        },
        description: playlist
            .description
            .map(|d| truncate_chars(&d.to_plaintext(), MAX_DESCRIPTION_CHARS))
            .unwrap_or_default(),
        visibility: Visibility::Private,
    };
    let videos: Vec<VideoUpsert> = videos.into_iter().filter_map(imported_video).collect();
    let created = state
        .playlists
        .create_with_items(user.id, &new, &videos, MAX_PLAYLISTS)
        .await
        .map_err(|e| map_limit(e, || format!("at most {MAX_PLAYLISTS} playlists allowed")))?;
    let body = playlist_dto(created, true, &|url| state.image_url(url));
    Ok((StatusCode::CREATED, Json(body)).into_response())
}

/// A playlist with its items. Private playlists are only visible to their owner; unlisted
/// ones to anyone with the ID, including anonymous users.
#[utoipa::path(
    get,
    path = "/playlists/{id}",
    tag = "playlists",
    params(("id" = String, Path, description = "Playlist ID")),
    responses(
        (status = OK, body = PlaylistDetails),
        (status = NOT_FOUND, description = "No such playlist, or not visible", body = ErrorBody),
    )
)]
async fn get_playlist(
    State(state): State<AppState>,
    OptionalUser(user): OptionalUser,
    Path(id): Path<String>,
) -> Result<Json<PlaylistDetails>, ApiError> {
    check_playlist_id(&id)?;
    let playlist = state.playlists.get(&id).await?.ok_or_else(not_found)?;
    let is_owner = user.is_some_and(|u| u.id == playlist.user_id);
    if !is_owner && playlist.visibility != Visibility::Unlisted {
        return Err(not_found());
    }
    let items = state.playlists.items(&id).await?;
    let img = |url: &str| state.image_url(url);
    Ok(Json(PlaylistDetails {
        owner: playlist.owner.clone(),
        playlist: playlist_dto(playlist, is_owner, &img),
        is_owner,
        items: items.into_iter().map(|i| entry_dto(i, &img)).collect(),
    }))
}

/// Change title, description or visibility.
#[utoipa::path(
    patch,
    path = "/playlists/{id}",
    tag = "playlists",
    security(("session" = [])),
    params(("id" = String, Path, description = "Playlist ID")),
    request_body = UpdatePlaylist,
    responses(
        (status = OK, body = Playlist),
        (status = UNAUTHORIZED, description = "Not signed in", body = ErrorBody),
        (status = FORBIDDEN, description = "Foreign origin", body = ErrorBody),
        (status = NOT_FOUND, description = "No such playlist of the user", body = ErrorBody),
        (status = UNPROCESSABLE_ENTITY, description = "Invalid title or description", body = ErrorBody),
    )
)]
async fn update_playlist(
    State(state): State<AppState>,
    CurrentUser(user): CurrentUser,
    Path(id): Path<String>,
    body: Result<Json<UpdatePlaylist>, JsonRejection>,
) -> Result<Json<Playlist>, ApiError> {
    check_playlist_id(&id)?;
    let Json(body) = body?;
    let mut fields = BTreeMap::new();
    let update = PlaylistUpdate {
        title: body.title.map(|t| validate_title(&t, &mut fields)),
        description: body
            .description
            .map(|d| validate_description(&d, &mut fields)),
        visibility: body.visibility.map(Into::into),
    };
    check_fields(fields)?;
    let updated = state
        .playlists
        .update(user.id, &id, &update)
        .await?
        .ok_or_else(not_found)?;
    Ok(Json(playlist_dto(updated, true, &|url| {
        state.image_url(url)
    })))
}

/// Delete a playlist.
#[utoipa::path(
    delete,
    path = "/playlists/{id}",
    tag = "playlists",
    security(("session" = [])),
    params(("id" = String, Path, description = "Playlist ID")),
    responses(
        (status = NO_CONTENT, description = "Deleted"),
        (status = UNAUTHORIZED, description = "Not signed in", body = ErrorBody),
        (status = FORBIDDEN, description = "Foreign origin", body = ErrorBody),
        (status = NOT_FOUND, description = "No such playlist of the user", body = ErrorBody),
    )
)]
async fn delete_playlist(
    State(state): State<AppState>,
    CurrentUser(user): CurrentUser,
    Path(id): Path<String>,
) -> Result<StatusCode, ApiError> {
    check_playlist_id(&id)?;
    if state.playlists.delete(user.id, &id).await? {
        Ok(StatusCode::NO_CONTENT)
    } else {
        Err(not_found())
    }
}

/// Append a video. Idempotent; a video appears at most once per playlist.
#[utoipa::path(
    put,
    path = "/playlists/{id}/items/{video_id}",
    tag = "playlists",
    security(("session" = [])),
    params(
        ("id" = String, Path, description = "Playlist ID"),
        ("video_id" = String, Path, description = "YouTube video ID"),
        ("Accept-Language" = Option<String>, Header, description = "Content language"),
    ),
    responses(
        (status = CREATED, description = "Added", body = Playlist),
        (status = OK, description = "Was already in the playlist", body = Playlist),
        (status = BAD_REQUEST, description = "Invalid video ID", body = ErrorBody),
        (status = UNAUTHORIZED, description = "Not signed in", body = ErrorBody),
        (status = FORBIDDEN, description = "Foreign origin", body = ErrorBody),
        (status = NOT_FOUND, description = "No such playlist of the user, or no such video", body = ErrorBody),
        (status = CONFLICT, description = "Playlist is full", body = ErrorBody),
        (status = BAD_GATEWAY, description = "YouTube request failed", body = ErrorBody),
        (status = SERVICE_UNAVAILABLE, description = "Rate limited by YouTube", body = ErrorBody),
    )
)]
async fn add_item(
    State(state): State<AppState>,
    CurrentUser(user): CurrentUser,
    Path((id, video_id)): Path<(String, String)>,
    ContentLanguage(lang): ContentLanguage,
) -> Result<Response, ApiError> {
    check_playlist_id(&id)?;
    check_video_id(&video_id)?;
    // Checked before asking YouTube about the video.
    let playlist = state.playlists.get(&id).await?;
    if playlist.is_none_or(|p| p.user_id != user.id) {
        return Err(not_found());
    }
    Box::pin(ensure_cached(&state, &video_id, lang)).await?;
    let added = state
        .playlists
        .add_item(user.id, &id, &video_id, MAX_ITEMS)
        .await
        .map_err(|e| map_limit(e, || format!("at most {MAX_ITEMS} videos per playlist")))?
        .ok_or_else(not_found)?;
    let playlist = state.playlists.get(&id).await?.ok_or_else(not_found)?;
    let status = if added {
        StatusCode::CREATED
    } else {
        StatusCode::OK
    };
    let body = playlist_dto(playlist, true, &|url| state.image_url(url));
    Ok((status, Json(body)).into_response())
}

/// Remove a video. Succeeds if it is not in the playlist, too.
#[utoipa::path(
    delete,
    path = "/playlists/{id}/items/{video_id}",
    tag = "playlists",
    security(("session" = [])),
    params(
        ("id" = String, Path, description = "Playlist ID"),
        ("video_id" = String, Path, description = "YouTube video ID"),
    ),
    responses(
        (status = NO_CONTENT, description = "Not in the playlist anymore"),
        (status = BAD_REQUEST, description = "Invalid video ID", body = ErrorBody),
        (status = UNAUTHORIZED, description = "Not signed in", body = ErrorBody),
        (status = FORBIDDEN, description = "Foreign origin", body = ErrorBody),
        (status = NOT_FOUND, description = "No such playlist of the user", body = ErrorBody),
    )
)]
async fn remove_item(
    State(state): State<AppState>,
    CurrentUser(user): CurrentUser,
    Path((id, video_id)): Path<(String, String)>,
) -> Result<StatusCode, ApiError> {
    check_playlist_id(&id)?;
    check_video_id(&video_id)?;
    state
        .playlists
        .remove_item(user.id, &id, &video_id)
        .await?
        .ok_or_else(not_found)?;
    Ok(StatusCode::NO_CONTENT)
}

/// Move a video to another position.
#[utoipa::path(
    put,
    path = "/playlists/{id}/items/{video_id}/position",
    tag = "playlists",
    security(("session" = [])),
    params(
        ("id" = String, Path, description = "Playlist ID"),
        ("video_id" = String, Path, description = "YouTube video ID"),
    ),
    request_body = MoveItem,
    responses(
        (status = NO_CONTENT, description = "Moved"),
        (status = BAD_REQUEST, description = "Invalid video ID", body = ErrorBody),
        (status = UNAUTHORIZED, description = "Not signed in", body = ErrorBody),
        (status = FORBIDDEN, description = "Foreign origin", body = ErrorBody),
        (status = NOT_FOUND, description = "No such playlist of the user, or video not in it", body = ErrorBody),
    )
)]
async fn move_item(
    State(state): State<AppState>,
    CurrentUser(user): CurrentUser,
    Path((id, video_id)): Path<(String, String)>,
    body: Result<Json<MoveItem>, JsonRejection>,
) -> Result<StatusCode, ApiError> {
    check_playlist_id(&id)?;
    check_video_id(&video_id)?;
    let Json(body) = body?;
    match state
        .playlists
        .move_item(user.id, &id, &video_id, body.index)
        .await?
    {
        Some(true) => Ok(StatusCode::NO_CONTENT),
        _ => Err(not_found()),
    }
}

/// Remember which item the owner is playing, for continuing later.
#[utoipa::path(
    put,
    path = "/playlists/{id}/last-played",
    tag = "playlists",
    security(("session" = [])),
    params(("id" = String, Path, description = "Playlist ID")),
    request_body = LastPlayed,
    responses(
        (status = NO_CONTENT, description = "Remembered"),
        (status = BAD_REQUEST, description = "Invalid video ID", body = ErrorBody),
        (status = UNAUTHORIZED, description = "Not signed in", body = ErrorBody),
        (status = FORBIDDEN, description = "Foreign origin", body = ErrorBody),
        (status = NOT_FOUND, description = "No such playlist of the user, or video not in it", body = ErrorBody),
    )
)]
async fn set_last_played(
    State(state): State<AppState>,
    CurrentUser(user): CurrentUser,
    Path(id): Path<String>,
    body: Result<Json<LastPlayed>, JsonRejection>,
) -> Result<StatusCode, ApiError> {
    check_playlist_id(&id)?;
    let Json(body) = body?;
    check_video_id(&body.video_id)?;
    if state
        .playlists
        .set_last_played(user.id, &id, &body.video_id)
        .await?
    {
        Ok(StatusCode::NO_CONTENT)
    } else {
        Err(not_found())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn generated_ids_are_valid_and_distinct() {
        let a = new_playlist_id();
        let b = new_playlist_id();
        assert!(is_playlist_id(&a), "{a}");
        assert_ne!(a, b);
    }

    #[test]
    fn rejects_foreign_ids() {
        for id in [
            "",
            "PLrAXtmErZgOeiKm4sgNOknGvNjby9efdf",
            "RDdQw4w9WgXcQ",
            "abc def ghi jklm",
            "aaaaaaaaaaaaaaa",
            "aaaaaaaaaaaaaaaaa",
        ] {
            assert!(!is_playlist_id(id), "{id}");
        }
    }

    #[test]
    fn truncates_imported_text_by_characters() {
        assert_eq!(truncate_chars("  äöü  ", 2), "äö");
        assert_eq!(truncate_chars("ab cd", 3), "ab");
        assert_eq!(truncate_chars("short", 150), "short");
    }

    #[test]
    fn validates_titles() {
        let mut fields = BTreeMap::new();
        assert_eq!(validate_title("  Mix  ", &mut fields), "Mix");
        assert!(fields.is_empty());
        validate_title("   ", &mut fields);
        assert_eq!(fields.get("title"), Some(&FieldError::Required));
        let mut fields = BTreeMap::new();
        validate_title(&"ä".repeat(MAX_TITLE_CHARS + 1), &mut fields);
        assert_eq!(fields.get("title"), Some(&FieldError::TooLong));
        let mut fields = BTreeMap::new();
        validate_title(&"ä".repeat(MAX_TITLE_CHARS), &mut fields);
        assert!(fields.is_empty(), "counts characters, not bytes");
    }
}
