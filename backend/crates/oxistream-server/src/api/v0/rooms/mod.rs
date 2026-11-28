use axum::Json;
use axum::extract::rejection::JsonRejection;
use axum::extract::{Path, State};
use axum::http::{HeaderMap, StatusCode, header};
use axum::response::{IntoResponse, Response};
use axum::routing::get;
use base64::Engine;
use base64::engine::general_purpose::URL_SAFE_NO_PAD;
use serde::{Deserialize, Serialize};
use time::OffsetDateTime;
use utoipa::ToSchema;
use utoipa_axum::router::OpenApiRouter;
use utoipa_axum::routes;

use super::playlists::{check_video_id, ensure_cached, video_thumbnails};
use crate::AppState;
use crate::api::dto::Thumbnail;
use crate::api::error::{ApiError, ErrorBody, ErrorCode};
use crate::api::lang::ContentLanguage;
use crate::auth::{CurrentUser, OptionalUser, party_guest};
use crate::rooms::MAX_ROOMS_PER_USER;

pub mod protocol;
mod socket;

const ID_BYTES: usize = 16;
const ID_LEN: usize = 22;

pub fn router() -> OpenApiRouter<AppState> {
    OpenApiRouter::new()
        .routes(routes!(list_rooms, create_room))
        .routes(routes!(get_room, delete_room))
        .routes(routes!(admit_guest, release_guest))
        .route("/rooms/{id}/ws", get(socket::connect))
}

/// A new random 128-bit room ID: 22 URL-safe characters.
fn new_room_id() -> String {
    let mut bytes = [0u8; ID_BYTES];
    getrandom::fill(&mut bytes).expect("system random number generator");
    URL_SAFE_NO_PAD.encode(bytes)
}

pub(crate) fn is_room_id(id: &str) -> bool {
    id.len() == ID_LEN
        && id
            .bytes()
            .all(|b| b.is_ascii_alphanumeric() || b == b'-' || b == b'_')
}

fn not_found() -> ApiError {
    ApiError::new(StatusCode::NOT_FOUND, ErrorCode::NotFound, "room not found")
}

/// Malformed IDs cannot exist, so they get the same 404 as unknown ones.
fn check_room_id(id: &str) -> Result<(), ApiError> {
    if is_room_id(id) {
        Ok(())
    } else {
        Err(not_found())
    }
}

/// A watch together room.
#[derive(Debug, Serialize, ToSchema)]
pub struct RoomInfo {
    /// Random ID, used in `/party/{id}`.
    pub id: String,
    /// Username of the owner.
    pub owner: String,
    /// Whether the signed-in user created the room.
    pub is_owner: bool,
    #[serde(with = "time::serde::rfc3339")]
    pub created_at: OffsetDateTime,
}

/// One of the signed-in user's rooms.
#[derive(Debug, Serialize, ToSchema)]
pub struct RoomListItem {
    pub id: String,
    /// Connected participants.
    pub participant_count: u32,
    /// Current and upcoming entries.
    pub queue_count: u32,
    /// Title of the current video, if any.
    pub current_title: Option<String>,
    /// Of the current video; empty without one.
    pub thumbnails: Vec<Thumbnail>,
    #[serde(with = "time::serde::rfc3339")]
    pub created_at: OffsetDateTime,
    /// Last time anyone was in the room; it expires a day after.
    #[serde(with = "time::serde::rfc3339")]
    pub active_at: OffsetDateTime,
}

#[derive(Debug, Deserialize, ToSchema)]
pub struct CreateRoom {
    /// YouTube video to start with.
    #[serde(default)]
    pub video_id: Option<String>,
}

/// List the signed-in user's open rooms, newest first.
#[utoipa::path(
    get,
    path = "/rooms",
    tag = "rooms",
    security(("session" = [])),
    responses(
        (status = OK, description = "The user's rooms", body = [RoomListItem]),
        (status = UNAUTHORIZED, description = "Not signed in", body = ErrorBody),
    )
)]
async fn list_rooms(
    State(state): State<AppState>,
    CurrentUser(user): CurrentUser,
) -> Result<Json<Vec<RoomListItem>>, ApiError> {
    let rooms = state.rooms.repo().list_owned(user.id).await?;
    let img = |url: &str| state.image_url(url);
    Ok(Json(
        rooms
            .into_iter()
            .map(|r| RoomListItem {
                thumbnails: r
                    .current_video_id
                    .as_deref()
                    .map(|id| video_thumbnails(id, &img))
                    .unwrap_or_default(),
                participant_count: u32::try_from(r.participant_count).unwrap_or(u32::MAX),
                queue_count: u32::try_from(r.queue_count).unwrap_or(u32::MAX),
                current_title: r.current_title,
                created_at: r.created_at,
                active_at: r.active_at,
                id: r.id,
            })
            .collect(),
    ))
}

/// Open a room. Anyone with its link can join through the WebSocket at
/// `/api/v0/rooms/{id}/ws`, speaking `RoomClientMessage` and `RoomServerMessage`. Rooms
/// expire a day after the last participant left.
#[utoipa::path(
    post,
    path = "/rooms",
    tag = "rooms",
    security(("session" = [])),
    request_body = CreateRoom,
    responses(
        (status = CREATED, description = "Room opened", body = RoomInfo),
        (status = BAD_REQUEST, description = "Invalid video ID", body = ErrorBody),
        (status = UNAUTHORIZED, description = "Not signed in", body = ErrorBody),
        (status = FORBIDDEN, description = "Foreign origin", body = ErrorBody),
        (status = NOT_FOUND, description = "Video not found", body = ErrorBody),
        (status = CONFLICT, description = "Too many open rooms", body = ErrorBody),
    )
)]
async fn create_room(
    State(state): State<AppState>,
    CurrentUser(user): CurrentUser,
    ContentLanguage(lang): ContentLanguage,
    body: Result<Json<CreateRoom>, JsonRejection>,
) -> Result<(StatusCode, Json<RoomInfo>), ApiError> {
    let Json(body) = body?;
    if let Some(video_id) = &body.video_id {
        check_video_id(video_id)?;
        Box::pin(ensure_cached(&state, video_id, lang)).await?;
    }
    let id = new_room_id();
    let video = body
        .video_id
        .as_deref()
        .map(|v| (v, user.username.as_str()));
    let room = state
        .rooms
        .repo()
        .create(user.id, &id, video, MAX_ROOMS_PER_USER)
        .await
        .map_err(|err| match err {
            oxistream_db::Error::LimitReached => ApiError::new(
                StatusCode::CONFLICT,
                ErrorCode::LimitReached,
                format!("at most {MAX_ROOMS_PER_USER} open rooms"),
            ),
            err => err.into(),
        })?;
    Ok((
        StatusCode::CREATED,
        Json(RoomInfo {
            id: room.id,
            owner: room.owner,
            is_owner: true,
            created_at: room.created_at,
        }),
    ))
}

/// Look up a room before joining it.
#[utoipa::path(
    get,
    path = "/rooms/{id}",
    tag = "rooms",
    params(("id" = String, Path, description = "Room ID")),
    responses(
        (status = OK, description = "The room", body = RoomInfo),
        (status = NOT_FOUND, description = "No such room", body = ErrorBody),
    )
)]
async fn get_room(
    State(state): State<AppState>,
    OptionalUser(user): OptionalUser,
    Path(id): Path<String>,
) -> Result<Json<RoomInfo>, ApiError> {
    check_room_id(&id)?;
    let room = state.rooms.repo().get(&id).await?.ok_or_else(not_found)?;
    Ok(Json(RoomInfo {
        is_owner: user.is_some_and(|u| u.id == room.owner_id),
        id: room.id,
        owner: room.owner,
        created_at: room.created_at,
    }))
}

/// Close a room; everyone in it is disconnected.
#[utoipa::path(
    delete,
    path = "/rooms/{id}",
    tag = "rooms",
    security(("session" = [])),
    params(("id" = String, Path, description = "Room ID")),
    responses(
        (status = NO_CONTENT, description = "Room closed"),
        (status = UNAUTHORIZED, description = "Not signed in", body = ErrorBody),
        (status = FORBIDDEN, description = "Foreign origin", body = ErrorBody),
        (status = NOT_FOUND, description = "No such room of the user", body = ErrorBody),
    )
)]
async fn delete_room(
    State(state): State<AppState>,
    CurrentUser(user): CurrentUser,
    Path(id): Path<String>,
) -> Result<StatusCode, ApiError> {
    check_room_id(&id)?;
    if state.rooms.repo().delete(user.id, &id).await? {
        Ok(StatusCode::NO_CONTENT)
    } else {
        Err(not_found())
    }
}

#[derive(Debug, Deserialize, ToSchema)]
pub struct GuestCredential {
    /// The key the guest joined the room's WebSocket with.
    pub guest_key: String,
}

/// Confirm that a guest is in the room. Sets the `oxistream_party` cookie, with which the guest
/// may use the app like an anonymous user while `OXISTREAM_REQUIRE_LOGIN` is set, as long as it
/// stays in the room. Clients call this after joining and every 30 s.
#[utoipa::path(
    post,
    path = "/rooms/{id}/guest",
    tag = "rooms",
    params(("id" = String, Path, description = "Room ID")),
    request_body = GuestCredential,
    responses(
        (status = NO_CONTENT, description = "Cookie set"),
        (status = BAD_REQUEST, description = "Malformed body", body = ErrorBody),
        (status = FORBIDDEN, description = "Foreign origin", body = ErrorBody),
        (status = NOT_FOUND, description = "No such room, or the guest is not in it", body = ErrorBody),
    )
)]
async fn admit_guest(
    State(state): State<AppState>,
    Path(id): Path<String>,
    body: Result<Json<GuestCredential>, JsonRejection>,
) -> Result<Response, ApiError> {
    check_room_id(&id)?;
    let Json(body) = body?;
    let key = socket::guest_participant_key(&body.guest_key).ok_or_else(not_found)?;
    if state.rooms.repo().presence(&id, &key).await? != oxistream_db::Presence::Present {
        return Err(not_found());
    }
    let value = state.auth.party_guests().encode(&party_guest::PartyGuest {
        room: id,
        key,
        seen: party_guest::now(),
    });
    let cookie = party_guest::cookie(&value, state.auth.config().cookie_secure);
    Ok((StatusCode::NO_CONTENT, [(header::SET_COOKIE, cookie)]).into_response())
}

/// Give up guest access, e.g. after leaving the room.
#[utoipa::path(
    delete,
    path = "/rooms/{id}/guest",
    tag = "rooms",
    params(("id" = String, Path, description = "Room ID")),
    responses(
        (status = NO_CONTENT, description = "Cookie cleared"),
        (status = FORBIDDEN, description = "Foreign origin", body = ErrorBody),
    )
)]
async fn release_guest(State(state): State<AppState>, headers: HeaderMap) -> Response {
    let guests = state.auth.party_guests();
    if let Some(guest) = party_guest::from_headers(&headers).and_then(|v| guests.decode(v)) {
        guests.forget(&guest.room, &guest.key).await;
    }
    let cookie = party_guest::clear_cookie(state.auth.config().cookie_secure);
    (StatusCode::NO_CONTENT, [(header::SET_COOKIE, cookie)]).into_response()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn room_ids_are_long_and_url_safe() {
        let id = new_room_id();
        assert!(is_room_id(&id), "{id}");
        assert_ne!(id, new_room_id());
        assert!(!is_room_id("short"));
        assert!(!is_room_id("aaaaaaaaaaaaaaaaaaaaa/"));
    }
}
