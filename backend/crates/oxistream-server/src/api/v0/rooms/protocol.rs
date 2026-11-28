//! Messages of the watch room WebSocket, JSON text frames tagged by `type`.

use oxistream_db::{
    ChatMessage, ParticipantStatus, Permission, Place, Reaction, RoomEntry, RoomSnapshot,
    SyncState, Uuid,
};
use serde::{Deserialize, Serialize};
use utoipa::ToSchema;

use super::super::playlists::cached_video_summary;
use crate::api::dto::VideoSummary;

/// Sent by clients. The first message must be `join`; positions are in seconds.
#[derive(Debug, Clone, PartialEq, Deserialize, ToSchema)]
#[serde(tag = "type", rename_all = "snake_case", deny_unknown_fields)]
pub enum RoomClientMessage {
    Join {
        /// Display name of a guest, 1 to 32 characters; ignored for signed-in users.
        #[serde(default)]
        name: Option<String>,
        /// Random 16 to 128 URL-safe characters a guest keeps, so a removal sticks.
        #[serde(default)]
        guest_key: Option<String>,
    },
    /// Answered with `pong`, for estimating the server clock.
    Ping {
        t: f64,
    },
    Play {
        position: f64,
        /// Server time in ms since the epoch at which `position` was read, from the `pong`
        /// clock; at most 5 s in the past. Defaults to when the server receives the message.
        #[serde(default)]
        at: Option<f64>,
    },
    Pause {
        position: f64,
    },
    Seek {
        position: f64,
        /// Server time in ms since the epoch at which `position` was read, from the `pong`
        /// clock; at most 5 s in the past. Defaults to when the server receives the message.
        #[serde(default)]
        at: Option<f64>,
    },
    /// The current entry finished playing; the queue advances once, however many report it.
    Ended {
        entry: i64,
    },
    /// Skip the current entry.
    Skip {
        entry: i64,
    },
    /// Play a queued entry now.
    Select {
        entry: i64,
    },
    Add {
        video_id: String,
        /// Plays it now instead of queueing it; the current entry is dropped.
        #[serde(default)]
        play: bool,
    },
    /// Queues up to 100 videos in order, as many as fit; answered with `added`.
    AddMany {
        video_ids: Vec<String>,
    },
    Remove {
        entry: i64,
    },
    /// Moves an upcoming entry to index `to` of `queue`; `revision` is that of the state it
    /// was based on. A newer room answers `stale` and stays as it is.
    Move {
        entry: i64,
        to: u32,
        revision: i64,
    },
    /// Puts the upcoming entries in a random order.
    Shuffle,
    /// When the queue runs out, append a related video of the last entry that the room has not
    /// queued before.
    Autoplay {
        enabled: bool,
    },
    /// Owner only: removes a participant for the rest of the room's life.
    Kick {
        participant: Uuid,
    },
    /// Owner only: who may control playback and the queue, and who may add videos. Omitted
    /// fields keep their setting.
    Permissions {
        #[serde(default)]
        control: Option<RoomPermission>,
        #[serde(default)]
        add: Option<RoomPermission>,
    },
    /// A chat message of up to 500 characters; whitespace is collapsed.
    Chat {
        text: String,
    },
    /// Shown briefly over everyone's player.
    React {
        reaction: RoomReaction,
    },
    /// What this participant's player and page are doing, for everyone's member list. Send it
    /// on change; the server keeps the latest and passes it on at most once per second.
    Status {
        /// `null` while there is nothing to play.
        #[serde(default)]
        sync: Option<MemberSync>,
        place: MemberPlace,
    },
}

/// Who besides the owner may do something.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, ToSchema)]
#[serde(rename_all = "snake_case")]
pub enum RoomPermission {
    Everyone,
    Host,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, ToSchema)]
#[serde(rename_all = "snake_case")]
pub enum RoomReaction {
    Heart,
    Laugh,
    Wow,
    Sad,
    Clap,
    Fire,
}

impl From<RoomPermission> for Permission {
    fn from(permission: RoomPermission) -> Self {
        match permission {
            RoomPermission::Everyone => Self::Everyone,
            RoomPermission::Host => Self::Host,
        }
    }
}

impl From<Permission> for RoomPermission {
    fn from(permission: Permission) -> Self {
        match permission {
            Permission::Everyone => Self::Everyone,
            Permission::Host => Self::Host,
        }
    }
}

impl From<RoomReaction> for Reaction {
    fn from(reaction: RoomReaction) -> Self {
        match reaction {
            RoomReaction::Heart => Self::Heart,
            RoomReaction::Laugh => Self::Laugh,
            RoomReaction::Wow => Self::Wow,
            RoomReaction::Sad => Self::Sad,
            RoomReaction::Clap => Self::Clap,
            RoomReaction::Fire => Self::Fire,
        }
    }
}

impl From<Reaction> for RoomReaction {
    fn from(reaction: Reaction) -> Self {
        match reaction {
            Reaction::Heart => Self::Heart,
            Reaction::Laugh => Self::Laugh,
            Reaction::Wow => Self::Wow,
            Reaction::Sad => Self::Sad,
            Reaction::Clap => Self::Clap,
            Reaction::Fire => Self::Fire,
        }
    }
}

/// How well a participant's player keeps up with the room.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, ToSchema)]
#[serde(rename_all = "snake_case")]
pub enum MemberSync {
    Synced,
    Buffering,
    /// More than a second off, or unable to play.
    Behind,
}

/// Where a participant is in the app.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, ToSchema)]
#[serde(rename_all = "snake_case")]
pub enum MemberPlace {
    /// On the watch page of the room's video.
    Watching,
    Browsing,
    Shorts,
}

impl From<MemberSync> for SyncState {
    fn from(sync: MemberSync) -> Self {
        match sync {
            MemberSync::Synced => Self::Synced,
            MemberSync::Buffering => Self::Buffering,
            MemberSync::Behind => Self::Behind,
        }
    }
}

impl From<SyncState> for MemberSync {
    fn from(sync: SyncState) -> Self {
        match sync {
            SyncState::Synced => Self::Synced,
            SyncState::Buffering => Self::Buffering,
            SyncState::Behind => Self::Behind,
        }
    }
}

impl From<MemberPlace> for Place {
    fn from(place: MemberPlace) -> Self {
        match place {
            MemberPlace::Watching => Self::Watching,
            MemberPlace::Browsing => Self::Browsing,
            MemberPlace::Shorts => Self::Shorts,
        }
    }
}

impl From<Place> for MemberPlace {
    fn from(place: Place) -> Self {
        match place {
            Place::Watching => Self::Watching,
            Place::Browsing => Self::Browsing,
            Place::Shorts => Self::Shorts,
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, ToSchema)]
#[serde(rename_all = "snake_case")]
pub enum RoomErrorCode {
    /// No such room, or it was closed.
    NotFound,
    /// The room has the maximum number of participants.
    RoomFull,
    QueueFull,
    /// A malformed message, name or video ID.
    Invalid,
    /// The video cannot be found or played.
    VideoUnavailable,
    /// Only the owner may do this, always or by the room's permissions.
    Forbidden,
    RateLimited,
    /// The room changed since the state a `move` was based on.
    Stale,
    Internal,
}

/// Sent by the server.
#[derive(Debug, Clone, Serialize, ToSchema)]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum RoomServerMessage {
    /// Answer to `join`, followed by `state`.
    Welcome {
        participant_id: Uuid,
        is_owner: bool,
        /// Milliseconds since the Unix epoch.
        server_time: i64,
    },
    /// The whole room, after every change.
    State {
        room: Box<RoomState>,
    },
    /// Played entries, newest first. Sent before the first `state` and whenever they change.
    History {
        entries: Vec<QueueEntry>,
    },
    Pong {
        t: f64,
        /// Milliseconds since the Unix epoch.
        server_time: i64,
    },
    Error {
        code: RoomErrorCode,
    },
    /// Answer to `add_many`. `skipped` videos were unavailable or did not fit in the queue.
    Added {
        added: u32,
        skipped: u32,
    },
    /// The room's latest chat messages, oldest first. Sent before the first `state`.
    ChatHistory {
        messages: Vec<RoomChatMessage>,
    },
    /// A new chat message.
    Chat {
        message: RoomChatMessage,
    },
    Reaction {
        participant: Uuid,
        reaction: RoomReaction,
    },
    /// A participant's status changed. It replaces the one in `participants` if `revision` is
    /// higher than that one's `status_revision`.
    Status {
        participant: Uuid,
        sync: Option<MemberSync>,
        place: Option<MemberPlace>,
        revision: i64,
    },
    /// The owner removed this participant; the connection closes.
    Kicked,
    /// The room was closed or expired; the connection closes.
    Closed,
}

#[derive(Debug, Clone, Serialize, ToSchema)]
pub struct RoomState {
    pub id: String,
    /// Increases with every change.
    pub revision: i64,
    /// Username of the owner.
    pub owner: String,
    /// `null` once the queue has run out.
    pub current: Option<QueueEntry>,
    /// Upcoming entries in order.
    pub queue: Vec<QueueEntry>,
    pub playing: bool,
    /// Seconds into the current video at `position_at`; add the time since while playing.
    pub position: f64,
    /// Milliseconds since the Unix epoch.
    pub position_at: i64,
    pub participants: Vec<RoomParticipant>,
    pub autoplay: bool,
    /// What autoplay plays after `current`, once it is the last entry and a pick was found.
    pub up_next: Option<VideoSummary>,
    pub permissions: RoomPermissions,
}

/// What participants other than the owner may do.
#[derive(Debug, Clone, Copy, Serialize, ToSchema)]
pub struct RoomPermissions {
    /// Play, pause, seek, skip, pick, reorder, shuffle, toggle autoplay and remove others'
    /// entries.
    pub control: RoomPermission,
    /// Queue videos; playing one now needs `control` too.
    pub add: RoomPermission,
}

#[derive(Debug, Clone, Serialize, ToSchema)]
pub struct RoomChatMessage {
    pub id: i64,
    pub name: String,
    pub is_owner: bool,
    pub is_guest: bool,
    /// Sent by this participant, or before with the same account or guest key.
    pub own: bool,
    pub text: String,
    /// Milliseconds since the Unix epoch.
    pub sent_at: i64,
}

#[derive(Debug, Clone, Serialize, ToSchema)]
pub struct QueueEntry {
    pub id: i64,
    /// `channel` is always set; view counts and dates are not.
    pub video: VideoSummary,
    /// Display name of whoever queued it; empty for `autoplay` entries.
    pub added_by: String,
    /// Appended by autoplay.
    pub autoplay: bool,
    /// Queued by this participant, or before with the same account or guest key; they may
    /// always remove it.
    pub own: bool,
}

#[derive(Debug, Clone, Serialize, ToSchema)]
pub struct RoomParticipant {
    pub id: Uuid,
    pub name: String,
    pub is_owner: bool,
    pub is_guest: bool,
    /// `null` until reported, or while there is nothing to play.
    pub sync: Option<MemberSync>,
    /// `null` until reported.
    pub place: Option<MemberPlace>,
    /// Grows with every status change.
    pub status_revision: i64,
}

pub fn status_message(participant: Uuid, status: ParticipantStatus) -> RoomServerMessage {
    RoomServerMessage::Status {
        participant,
        sync: status.sync.map(Into::into),
        place: status.place.map(Into::into),
        revision: status.revision,
    }
}

pub fn unix_ms(at: time::OffsetDateTime) -> i64 {
    i64::try_from(at.unix_timestamp_nanos() / 1_000_000).unwrap_or(i64::MAX)
}

/// A chat message as seen by the participant with key `viewer`.
pub fn chat_message(m: &ChatMessage, owner_id: Uuid, viewer: &str) -> RoomChatMessage {
    RoomChatMessage {
        id: m.id,
        name: m.name.clone(),
        is_owner: m.author_key == format!("u:{owner_id}"),
        is_guest: m.author_key.starts_with("g:"),
        own: m.author_key == viewer,
        text: m.body.clone(),
        sent_at: unix_ms(m.sent_at),
    }
}

fn queue_entry(e: &RoomEntry, img: &impl Fn(&str) -> String, viewer: &str) -> QueueEntry {
    QueueEntry {
        id: e.entry_id,
        video: cached_video_summary(e.video.clone(), img),
        added_by: e.added_by.clone(),
        autoplay: e.autoplay,
        own: e.added_key.as_deref() == Some(viewer),
    }
}

/// Played entries as seen by the participant with key `viewer`.
pub fn history(
    snapshot: &RoomSnapshot,
    img: &impl Fn(&str) -> String,
    viewer: &str,
) -> Vec<QueueEntry> {
    snapshot
        .history
        .iter()
        .map(|e| queue_entry(e, img, viewer))
        .collect()
}

/// The room as seen by the participant with key `viewer`.
#[allow(clippy::cast_precision_loss)]
pub fn room_state(
    snapshot: &RoomSnapshot,
    img: &impl Fn(&str) -> String,
    viewer: &str,
) -> RoomState {
    let room = &snapshot.room;
    let (current, queue): (Vec<_>, Vec<_>) = snapshot
        .queue
        .iter()
        .map(|e| queue_entry(e, img, viewer))
        .partition(|e| Some(e.id) == room.current_entry_id);
    RoomState {
        id: room.id.clone(),
        revision: room.revision,
        owner: room.owner.clone(),
        current: current.into_iter().next(),
        queue,
        playing: room.playing,
        position: room.position_ms as f64 / 1000.0,
        position_at: unix_ms(room.position_at),
        participants: snapshot
            .participants
            .iter()
            .map(|p| RoomParticipant {
                id: p.id,
                name: p.display_name.clone(),
                is_owner: p.user_id == Some(room.owner_id),
                is_guest: p.user_id.is_none(),
                sync: p.status.sync.map(Into::into),
                place: p.status.place.map(Into::into),
                status_revision: p.status.revision,
            })
            .collect(),
        autoplay: room.autoplay,
        up_next: snapshot
            .up_next
            .clone()
            .map(|v| cached_video_summary(v, img)),
        permissions: RoomPermissions {
            control: room.control_permission.into(),
            add: room.add_permission.into(),
        },
    }
}
