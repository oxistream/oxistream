//! Messages of the watch room WebSocket, JSON text frames tagged by `type`.

use oxistream_db::{RoomSnapshot, Uuid};
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
    /// Only the owner may do this.
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
}

#[derive(Debug, Clone, Serialize, ToSchema)]
pub struct RoomParticipant {
    pub id: Uuid,
    pub name: String,
    pub is_owner: bool,
    pub is_guest: bool,
}

pub fn unix_ms(at: time::OffsetDateTime) -> i64 {
    i64::try_from(at.unix_timestamp_nanos() / 1_000_000).unwrap_or(i64::MAX)
}

#[allow(clippy::cast_precision_loss)]
pub fn room_state(snapshot: &RoomSnapshot, img: &impl Fn(&str) -> String) -> RoomState {
    let room = &snapshot.room;
    let (current, queue): (Vec<_>, Vec<_>) = snapshot
        .queue
        .iter()
        .map(|e| QueueEntry {
            id: e.entry_id,
            video: cached_video_summary(e.video.clone(), img),
            added_by: e.added_by.clone(),
            autoplay: e.autoplay,
        })
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
            })
            .collect(),
        autoplay: room.autoplay,
        up_next: snapshot
            .up_next
            .clone()
            .map(|v| cached_video_summary(v, img)),
    }
}
