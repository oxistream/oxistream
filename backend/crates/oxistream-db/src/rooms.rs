use std::str::FromStr;
use std::time::Duration;

use sqlx::postgres::{PgListener, PgNotification, PgRow};
use sqlx::{PgPool, Row};
use time::OffsetDateTime;
use uuid::Uuid;

use crate::{Error, Video, VideoUpsert};

/// Notifications carry the ID of a room that changed or was deleted.
pub const ROOM_CHANNEL: &str = "watch_rooms";
/// Notifications carry a participant's new status.
pub const STATUS_CHANNEL: &str = "watch_room_status";
/// Notifications carry a new chat message.
pub const CHAT_CHANNEL: &str = "watch_room_chat";
/// Notifications carry a reaction; reactions are not stored.
pub const REACTION_CHANNEL: &str = "watch_room_reaction";
/// Played entries a room keeps as its history.
pub const MAX_HISTORY: i64 = 100;
/// Chat messages a room keeps.
pub const MAX_MESSAGES: i64 = 100;
/// Longest chat message, in characters.
pub const MAX_MESSAGE_CHARS: usize = 500;

#[derive(Debug, Clone, PartialEq, Eq, sqlx::FromRow)]
pub struct Room {
    pub id: String,
    pub owner_id: Uuid,
    /// Username of the owner.
    pub owner: String,
    pub current_entry_id: Option<i64>,
    pub playing: bool,
    /// Position at `position_at`; advances with the clock while playing.
    pub position_ms: i64,
    pub position_at: OffsetDateTime,
    pub revision: i64,
    /// Participant keys the owner removed.
    pub banned: Vec<String>,
    pub created_at: OffsetDateTime,
    pub active_at: OffsetDateTime,
    pub autoplay: bool,
    /// The entry `autoplay_video_id` was picked for; with no video, nothing was found.
    pub autoplay_source: Option<i64>,
    pub autoplay_video_id: Option<String>,
    /// Who may play, pause, seek, skip, pick and reorder.
    #[sqlx(try_from = "String")]
    pub control_permission: Permission,
    /// Who may add videos.
    #[sqlx(try_from = "String")]
    pub add_permission: Permission,
}

/// Who besides the owner, who always may, may do something in a room.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum Permission {
    #[default]
    Everyone,
    Host,
}

#[derive(Debug, Clone, PartialEq, Eq, sqlx::FromRow)]
pub struct RoomEntry {
    pub entry_id: i64,
    /// Display name of whoever queued it.
    pub added_by: String,
    pub added_at: OffsetDateTime,
    /// Appended by autoplay rather than by a participant.
    pub autoplay: bool,
    /// Grows with every entry that finished; `None` until then.
    pub played: Option<i64>,
    /// Participant key of whoever queued it; `None` for autoplay.
    pub added_key: Option<String>,
    #[sqlx(flatten)]
    pub video: Video,
}

/// How well a participant's player keeps up with the room.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SyncState {
    Synced,
    Buffering,
    Behind,
}

/// Where a participant is in the app.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Place {
    /// On the watch page of the room's video.
    Watching,
    Browsing,
    Shorts,
}

macro_rules! text_enum {
    ($ty:ident { $($variant:ident => $text:literal),+ $(,)? }) => {
        impl $ty {
            pub fn as_str(self) -> &'static str {
                match self {
                    $(Self::$variant => $text),+
                }
            }
        }

        impl FromStr for $ty {
            type Err = String;

            fn from_str(s: &str) -> Result<Self, Self::Err> {
                match s {
                    $($text => Ok(Self::$variant),)+
                    _ => Err(format!("unknown {} {s:?}", stringify!($ty))),
                }
            }
        }
    };
}

text_enum!(SyncState { Synced => "synced", Buffering => "buffering", Behind => "behind" });
text_enum!(Place { Watching => "watching", Browsing => "browsing", Shorts => "shorts" });
text_enum!(Permission { Everyone => "everyone", Host => "host" });
text_enum!(Reaction {
    Heart => "heart",
    Laugh => "laugh",
    Wow => "wow",
    Sad => "sad",
    Clap => "clap",
    Fire => "fire",
});

impl TryFrom<String> for Permission {
    type Error = String;

    fn try_from(s: String) -> Result<Self, Self::Error> {
        s.parse()
    }
}

/// The fixed set of reactions participants may send.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Reaction {
    Heart,
    Laugh,
    Wow,
    Sad,
    Clap,
    Fire,
}

#[derive(Debug, Clone, PartialEq, Eq, sqlx::FromRow)]
pub struct ChatMessage {
    pub id: i64,
    pub room_id: String,
    /// Participant key of the author.
    pub author_key: String,
    /// Display name of the author when it was sent.
    pub name: String,
    pub body: String,
    pub sent_at: OffsetDateTime,
}

/// What a participant last reported about itself.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub struct ParticipantStatus {
    pub sync: Option<SyncState>,
    pub place: Option<Place>,
    /// Grows with every change, so the newest of several copies wins.
    pub revision: i64,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Participant {
    pub id: Uuid,
    /// `u:<user id>` or `g:<hash of the guest key>`.
    pub key: String,
    pub user_id: Option<Uuid>,
    pub display_name: String,
    pub joined_at: OffsetDateTime,
    pub status: ParticipantStatus,
}

fn text_column<T: FromStr<Err = String>>(row: &PgRow, column: &str) -> sqlx::Result<Option<T>> {
    row.try_get::<Option<String>, _>(column)?
        .map(|text| text.parse())
        .transpose()
        .map_err(|err: String| sqlx::Error::ColumnDecode {
            index: column.into(),
            source: err.into(),
        })
}

impl sqlx::FromRow<'_, PgRow> for Participant {
    fn from_row(row: &PgRow) -> sqlx::Result<Self> {
        Ok(Self {
            id: row.try_get("id")?,
            key: row.try_get("key")?,
            user_id: row.try_get("user_id")?,
            display_name: row.try_get("display_name")?,
            joined_at: row.try_get("joined_at")?,
            status: ParticipantStatus {
                sync: text_column(row, "sync")?,
                place: text_column(row, "place")?,
                revision: row.try_get("status_revision")?,
            },
        })
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct NewParticipant {
    pub id: Uuid,
    pub key: String,
    pub user_id: Option<Uuid>,
    pub display_name: String,
}

/// A room with its queue (current entry first, then the others in order), history (newest
/// first) and participants (by join time).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RoomSnapshot {
    pub room: Room,
    pub queue: Vec<RoomEntry>,
    /// Played entries, at most [`MAX_HISTORY`], one per video.
    pub history: Vec<RoomEntry>,
    pub participants: Vec<Participant>,
    /// What autoplay appends once the current entry, the last one, is over.
    pub up_next: Option<Video>,
}

impl RoomSnapshot {
    /// The current entry and its video, if autoplay has yet to pick what follows it.
    pub fn needs_pick(&self) -> Option<(i64, &str)> {
        let room = &self.room;
        let current = room.current_entry_id?;
        let [entry] = self.queue.as_slice() else {
            return None;
        };
        (room.autoplay && room.autoplay_source != Some(current) && entry.entry_id == current)
            .then_some((current, entry.video.id.as_str()))
    }
}

#[derive(Debug, Clone, PartialEq, Eq, sqlx::FromRow)]
pub struct RoomSummary {
    pub id: String,
    pub created_at: OffsetDateTime,
    pub active_at: OffsetDateTime,
    pub participant_count: i64,
    pub queue_count: i64,
    pub current_video_id: Option<String>,
    pub current_title: Option<String>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum JoinOutcome {
    Joined,
    NotFound,
    /// The owner removed this participant key.
    Banned,
    Full,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum RoomOp {
    Play {
        position_ms: i64,
    },
    Pause {
        position_ms: i64,
    },
    Seek {
        position_ms: i64,
    },
    /// Moves on to the next entry if `from` is still the current one, so an ended video
    /// reported by several clients advances the queue once.
    Advance {
        from: i64,
    },
    /// Plays an upcoming entry now; the current one goes to the history.
    Select {
        entry: i64,
    },
    /// Queues a cached video; it becomes current, paused, if nothing is. With `play`, it
    /// replaces the current entry and plays.
    Add {
        video_id: String,
        added_by: String,
        max_queue: u32,
        play: bool,
    },
    /// Queues cached videos in order, as many as fit; the first becomes current, paused, if
    /// nothing is. Fails with [`Error::LimitReached`] only if none fit.
    AddMany {
        video_ids: Vec<String>,
        added_by: String,
        max_queue: u32,
    },
    Remove {
        entry: i64,
    },
    /// Moves an upcoming entry to index `to` among the upcoming entries (clamped), if the room
    /// is still at `revision`.
    Move {
        entry: i64,
        to: u32,
        revision: i64,
    },
    /// Puts the upcoming entries in a random, different order.
    Shuffle,
    /// Removes a participant's connections and bans their key.
    Kick {
        participant: Uuid,
    },
    SetAutoplay {
        enabled: bool,
    },
    /// Owner only; `None` keeps the current setting.
    SetPermissions {
        control: Option<Permission>,
        add: Option<Permission>,
    },
    /// Stores the first of `candidates` the room has not queued yet as what autoplay appends
    /// after `source`, if that is still the current and last entry and nothing was picked for it.
    Pick {
        source: i64,
        candidates: Vec<VideoUpsert>,
    },
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum OpOutcome {
    Applied,
    /// Valid, but nothing to do, e.g. a stale [`RoomOp::Advance`].
    Unchanged,
    NotFound,
    /// The room changed since the revision the op was based on; nothing was done.
    Stale,
    /// [`RoomOp::AddMany`] queued this many videos; the rest did not fit.
    Added(u32),
    /// The room's permissions do not allow the participant this.
    Forbidden,
}

#[async_trait::async_trait]
pub trait RoomRepository: Send + Sync {
    /// Fails with [`Error::LimitReached`] if the owner already has `max_rooms`. A cached
    /// `video` becomes the current entry.
    async fn create(
        &self,
        owner_id: Uuid,
        id: &str,
        video: Option<(&str, &str)>,
        max_rooms: u32,
    ) -> Result<Room, Error>;

    async fn get(&self, id: &str) -> Result<Option<Room>, Error>;

    async fn snapshot(&self, id: &str) -> Result<Option<RoomSnapshot>, Error>;

    /// The owner's rooms, newest first.
    async fn list_owned(&self, owner_id: Uuid) -> Result<Vec<RoomSummary>, Error>;

    /// Returns whether the owner had such a room.
    async fn delete(&self, owner_id: Uuid, id: &str) -> Result<bool, Error>;

    async fn join(
        &self,
        room_id: &str,
        participant: &NewParticipant,
        max_participants: u32,
    ) -> Result<JoinOutcome, Error>;

    async fn leave(&self, participant_id: Uuid) -> Result<(), Error>;

    /// Stores what a participant reports about itself, if it changed, and tells all replicas.
    /// Neither the room's revision nor its activity change.
    async fn set_status(
        &self,
        participant_id: Uuid,
        sync: Option<SyncState>,
        place: Option<Place>,
    ) -> Result<(), Error>;

    /// Keeps participants and their rooms from being cleaned up.
    async fn heartbeat(&self, participants: &[Uuid], rooms: &[String]) -> Result<(), Error>;

    /// Applies an op as the server itself, without permission checks.
    async fn apply(
        &self,
        room_id: &str,
        op: &RoomOp,
        now: OffsetDateTime,
    ) -> Result<OpOutcome, Error> {
        self.apply_as(room_id, op, None, now).await
    }

    /// Applies an op for the participant with key `actor` if the room's permissions allow it,
    /// else answers [`OpOutcome::Forbidden`]. Fails with [`Error::LimitReached`] when adding to
    /// a full queue. Changes bump the revision. `now` becomes the position's reference time.
    async fn apply_as(
        &self,
        room_id: &str,
        op: &RoomOp,
        actor: Option<&str>,
        now: OffsetDateTime,
    ) -> Result<OpOutcome, Error>;

    /// Stores a chat message, dropping the oldest beyond [`MAX_MESSAGES`], and tells all
    /// replicas. `None` if there is no such room. The revision does not change.
    async fn post_message(
        &self,
        room_id: &str,
        author_key: &str,
        name: &str,
        body: &str,
    ) -> Result<Option<ChatMessage>, Error>;

    /// The room's chat, oldest first.
    async fn messages(&self, room_id: &str) -> Result<Vec<ChatMessage>, Error>;

    /// Tells all replicas about a reaction, without storing it.
    async fn react(
        &self,
        room_id: &str,
        participant_id: Uuid,
        reaction: Reaction,
    ) -> Result<(), Error>;

    /// Drops participants not seen for `stale_after` and rooms nobody was in for
    /// `expire_after`. Returns the number of deleted rooms.
    async fn cleanup(&self, stale_after: Duration, expire_after: Duration) -> Result<u64, Error>;
}

#[derive(Debug, Clone)]
pub struct PgRoomRepository {
    pool: PgPool,
}

impl PgRoomRepository {
    pub fn new(pool: PgPool) -> Self {
        Self { pool }
    }
}

const ROOM_SELECT: &str = "SELECT r.id, r.owner_id, u.username AS owner, r.current_entry_id, \
        r.playing, r.position_ms, r.position_at, r.revision, r.banned, r.created_at, r.active_at, \
        r.autoplay, r.autoplay_source, r.autoplay_video_id, r.control_permission, \
        r.add_permission \
    FROM watch_rooms r JOIN users u ON u.id = r.owner_id WHERE r.id = $1";

const ENTRY_SELECT: &str = "SELECT q.id AS entry_id, q.added_by, q.added_at, q.autoplay, \
        q.played, q.added_key, v.id, v.title, v.channel_id, v.channel_name, v.duration_seconds, \
        v.published_at, v.thumbnail_url, v.updated_at \
    FROM watch_room_queue q JOIN videos v ON v.id = q.video_id";

#[derive(sqlx::FromRow)]
struct Locked {
    #[sqlx(rename = "current_entry_id")]
    current: Option<i64>,
    revision: i64,
    autoplay: bool,
    autoplay_source: Option<i64>,
    autoplay_video_id: Option<String>,
    owner_id: Uuid,
    playing: bool,
    position_ms: i64,
    position_at: OffsetDateTime,
    #[sqlx(try_from = "String")]
    control_permission: Permission,
    #[sqlx(try_from = "String")]
    add_permission: Permission,
}

impl Locked {
    /// The video autoplay appends once `from` is over.
    fn pick_after(&self, from: i64) -> Option<&str> {
        (self.autoplay && self.autoplay_source == Some(from))
            .then_some(self.autoplay_video_id.as_deref())
            .flatten()
    }
}

async fn lock_room(tx: &mut sqlx::PgConnection, id: &str) -> Result<Option<Locked>, Error> {
    Ok(sqlx::query_as(
        "SELECT current_entry_id, revision, autoplay, autoplay_source, autoplay_video_id, \
            owner_id, playing, position_ms, position_at, control_permission, add_permission \
         FROM watch_rooms WHERE id = $1 FOR UPDATE",
    )
    .bind(id)
    .fetch_optional(&mut *tx)
    .await?)
}

async fn bump(tx: &mut sqlx::PgConnection, id: &str) -> Result<(), Error> {
    sqlx::query("UPDATE watch_rooms SET revision = revision + 1, active_at = now() WHERE id = $1")
        .bind(id)
        .execute(&mut *tx)
        .await?;
    Ok(())
}

async fn set_current(
    tx: &mut sqlx::PgConnection,
    id: &str,
    entry: Option<i64>,
    playing: bool,
    now: OffsetDateTime,
) -> Result<(), Error> {
    sqlx::query(
        "UPDATE watch_rooms SET current_entry_id = $2, playing = $3, position_ms = 0, \
            position_at = $4 WHERE id = $1",
    )
    .bind(id)
    .bind(entry)
    .bind(playing && entry.is_some())
    .bind(now)
    .execute(&mut *tx)
    .await?;
    Ok(())
}

/// Moves an entry to the history, replacing an older entry of the same video and dropping
/// the oldest beyond [`MAX_HISTORY`].
async fn retire(tx: &mut sqlx::PgConnection, id: &str, entry: i64) -> Result<(), Error> {
    sqlx::query(
        "WITH e AS (UPDATE watch_room_queue SET played = nextval('watch_room_played_seq') \
            WHERE id = $2 RETURNING id, video_id) \
         DELETE FROM watch_room_queue q USING e \
         WHERE q.room_id = $1 AND q.played IS NOT NULL AND q.video_id = e.video_id",
    )
    .bind(id)
    .bind(entry)
    .execute(&mut *tx)
    .await?;
    sqlx::query(
        "DELETE FROM watch_room_queue WHERE id IN (SELECT id FROM watch_room_queue \
            WHERE room_id = $1 AND played IS NOT NULL ORDER BY played DESC OFFSET $2)",
    )
    .bind(id)
    .bind(MAX_HISTORY)
    .execute(&mut *tx)
    .await?;
    Ok(())
}

/// Retires `from` and plays the first upcoming entry, else what autoplay picked, if anything.
async fn advance(
    tx: &mut sqlx::PgConnection,
    id: &str,
    locked: &Locked,
    from: i64,
    now: OffsetDateTime,
) -> Result<(), Error> {
    retire(tx, id, from).await?;
    let mut next: Option<i64> = sqlx::query_scalar(
        "SELECT id FROM watch_room_queue WHERE room_id = $1 AND played IS NULL \
         ORDER BY position, id LIMIT 1",
    )
    .bind(id)
    .fetch_optional(&mut *tx)
    .await?;
    if next.is_none()
        && let Some(video_id) = locked.pick_after(from)
    {
        next = Some(
            sqlx::query_scalar(
                "INSERT INTO watch_room_queue (room_id, video_id, added_by, autoplay) \
                 VALUES ($1, $2, '', true) RETURNING id",
            )
            .bind(id)
            .bind(video_id)
            .fetch_one(&mut *tx)
            .await?,
        );
    }
    set_current(tx, id, next, true, now).await
}

async fn set_position(
    tx: &mut sqlx::PgConnection,
    id: &str,
    playing: Option<bool>,
    position_ms: i64,
    now: OffsetDateTime,
) -> Result<(), Error> {
    sqlx::query(
        "UPDATE watch_rooms SET playing = COALESCE($2, playing), position_ms = $3, \
            position_at = $4 WHERE id = $1",
    )
    .bind(id)
    .bind(playing)
    .bind(position_ms)
    .bind(now)
    .execute(&mut *tx)
    .await?;
    Ok(())
}

async fn select(
    tx: &mut sqlx::PgConnection,
    id: &str,
    current: Option<i64>,
    entry: i64,
    now: OffsetDateTime,
) -> Result<bool, Error> {
    let exists: bool = sqlx::query_scalar(
        "SELECT EXISTS (SELECT 1 FROM watch_room_queue \
            WHERE id = $1 AND room_id = $2 AND played IS NULL)",
    )
    .bind(entry)
    .bind(id)
    .fetch_one(&mut *tx)
    .await?;
    if !exists || current == Some(entry) {
        return Ok(false);
    }
    if let Some(current) = current {
        retire(tx, id, current).await?;
    }
    set_current(tx, id, Some(entry), true, now).await?;
    Ok(true)
}

async fn add(
    tx: &mut sqlx::PgConnection,
    id: &str,
    current: Option<i64>,
    (video_id, added_by, max_queue, play): (&str, &str, u32, bool),
    added_key: Option<&str>,
    now: OffsetDateTime,
) -> Result<(), Error> {
    let count = queue_len(tx, id).await?;
    // Playing now drops the current entry, so the queue does not grow.
    let replaced = i64::from(play && current.is_some());
    if count - replaced >= i64::from(max_queue) {
        return Err(Error::LimitReached);
    }
    let entry: i64 = sqlx::query_scalar(
        "INSERT INTO watch_room_queue (room_id, video_id, added_by, added_key) \
         VALUES ($1, $2, $3, $4) RETURNING id",
    )
    .bind(id)
    .bind(video_id)
    .bind(added_by)
    .bind(added_key)
    .fetch_one(&mut *tx)
    .await?;
    if play {
        if let Some(current) = current {
            retire(tx, id, current).await?;
        }
        set_current(tx, id, Some(entry), true, now).await?;
    } else if current.is_none() {
        set_current(tx, id, Some(entry), false, now).await?;
    }
    Ok(())
}

/// The current and upcoming entries; the history does not count.
async fn queue_len(tx: &mut sqlx::PgConnection, id: &str) -> Result<i64, Error> {
    Ok(sqlx::query_scalar(
        "SELECT count(*) FROM watch_room_queue WHERE room_id = $1 AND played IS NULL",
    )
    .bind(id)
    .fetch_one(&mut *tx)
    .await?)
}

async fn add_many(
    tx: &mut sqlx::PgConnection,
    id: &str,
    current: Option<i64>,
    (video_ids, added_by, max_queue): (&[String], &str, u32),
    added_key: Option<&str>,
    now: OffsetDateTime,
) -> Result<u32, Error> {
    let free = i64::from(max_queue) - queue_len(tx, id).await?;
    let take = usize::try_from(free.max(0)).unwrap_or(usize::MAX);
    if take == 0 && !video_ids.is_empty() {
        return Err(Error::LimitReached);
    }
    let mut first = None;
    let mut added = 0;
    // One by one, so positions follow the given order.
    for video_id in video_ids.iter().take(take) {
        let entry: i64 = sqlx::query_scalar(
            "INSERT INTO watch_room_queue (room_id, video_id, added_by, added_key) \
             VALUES ($1, $2, $3, $4) RETURNING id",
        )
        .bind(id)
        .bind(video_id)
        .bind(added_by)
        .bind(added_key)
        .fetch_one(&mut *tx)
        .await?;
        first.get_or_insert(entry);
        added += 1;
    }
    if current.is_none() && first.is_some() {
        set_current(tx, id, first, false, now).await?;
    }
    Ok(added)
}

/// The upcoming entries in order, with their positions.
async fn upcoming(
    tx: &mut sqlx::PgConnection,
    id: &str,
    current: Option<i64>,
) -> Result<Vec<(i64, i64)>, Error> {
    Ok(sqlx::query_as(
        "SELECT id, position FROM watch_room_queue \
         WHERE room_id = $1 AND played IS NULL AND id IS DISTINCT FROM $2 \
         ORDER BY position, id",
    )
    .bind(id)
    .bind(current)
    .fetch_all(&mut *tx)
    .await?)
}

/// Gives the entries `order` the positions of `rows`, in turn.
async fn reorder(
    tx: &mut sqlx::PgConnection,
    rows: &[(i64, i64)],
    order: &[i64],
) -> Result<(), Error> {
    let positions: Vec<i64> = rows.iter().map(|(_, position)| *position).collect();
    sqlx::query(
        "UPDATE watch_room_queue q SET position = n.position \
         FROM unnest($1::bigint[], $2::bigint[]) AS n (id, position) WHERE q.id = n.id",
    )
    .bind(order)
    .bind(&positions)
    .execute(&mut *tx)
    .await?;
    Ok(())
}

async fn move_entry(
    tx: &mut sqlx::PgConnection,
    id: &str,
    current: Option<i64>,
    entry: i64,
    to: u32,
) -> Result<bool, Error> {
    let rows = upcoming(tx, id, current).await?;
    let Some(from) = rows.iter().position(|(e, _)| *e == entry) else {
        return Ok(false);
    };
    let to = usize::try_from(to)
        .unwrap_or(usize::MAX)
        .min(rows.len() - 1);
    if from == to {
        return Ok(false);
    }
    let mut order: Vec<i64> = rows.iter().map(|(e, _)| *e).collect();
    let moved = order.remove(from);
    order.insert(to, moved);
    reorder(tx, &rows, &order).await?;
    Ok(true)
}

async fn shuffle(
    tx: &mut sqlx::PgConnection,
    id: &str,
    current: Option<i64>,
) -> Result<bool, Error> {
    let rows = upcoming(tx, id, current).await?;
    if rows.len() < 2 {
        return Ok(false);
    }
    let ids: Vec<i64> = rows.iter().map(|(e, _)| *e).collect();
    // Retried while it comes out unchanged, which is likely for short queues.
    for _ in 0..8 {
        let order: Vec<i64> =
            sqlx::query_scalar("SELECT id FROM unnest($1::bigint[]) AS id ORDER BY random()")
                .bind(&ids)
                .fetch_all(&mut *tx)
                .await?;
        if order != ids {
            reorder(tx, &rows, &order).await?;
            return Ok(true);
        }
    }
    Ok(false)
}

async fn kick(tx: &mut sqlx::PgConnection, id: &str, participant: Uuid) -> Result<bool, Error> {
    let key: Option<String> = sqlx::query_scalar(
        "SELECT key FROM watch_room_participants WHERE id = $1 AND room_id = $2",
    )
    .bind(participant)
    .bind(id)
    .fetch_optional(&mut *tx)
    .await?;
    let Some(key) = key else {
        return Ok(false);
    };
    sqlx::query(
        "UPDATE watch_rooms SET banned = array_append(banned, $2) \
         WHERE id = $1 AND NOT $2 = ANY (banned)",
    )
    .bind(id)
    .bind(&key)
    .execute(&mut *tx)
    .await?;
    sqlx::query("DELETE FROM watch_room_participants WHERE room_id = $1 AND key = $2")
        .bind(id)
        .bind(&key)
        .execute(&mut *tx)
        .await?;
    Ok(true)
}

async fn set_autoplay(tx: &mut sqlx::PgConnection, id: &str, enabled: bool) -> Result<bool, Error> {
    let updated = sqlx::query(
        "UPDATE watch_rooms SET autoplay = $2 WHERE id = $1 AND autoplay IS DISTINCT FROM $2",
    )
    .bind(id)
    .bind(enabled)
    .execute(&mut *tx)
    .await?;
    Ok(updated.rows_affected() > 0)
}

async fn pick(
    tx: &mut sqlx::PgConnection,
    id: &str,
    locked: &Locked,
    source: i64,
    candidates: &[VideoUpsert],
) -> Result<bool, Error> {
    if !locked.autoplay
        || locked.current != Some(source)
        || locked.autoplay_source == Some(source)
        || !upcoming(tx, id, locked.current).await?.is_empty()
    {
        return Ok(false);
    }
    let ids: Vec<&str> = candidates.iter().map(|c| c.id.as_str()).collect();
    let seen: Vec<String> = sqlx::query_scalar(
        "SELECT video_id FROM watch_room_seen WHERE room_id = $1 AND video_id = ANY($2)",
    )
    .bind(id)
    .bind(&ids)
    .fetch_all(&mut *tx)
    .await?;
    let chosen = candidates.iter().find(|c| !seen.contains(&c.id));
    if let Some(video) = chosen {
        crate::videos::upsert(&mut *tx, video).await?;
    }
    sqlx::query(
        "UPDATE watch_rooms SET autoplay_source = $2, autoplay_video_id = $3 WHERE id = $1",
    )
    .bind(id)
    .bind(source)
    .bind(chosen.map(|c| c.id.as_str()))
    .execute(&mut *tx)
    .await?;
    Ok(true)
}

async fn set_permissions(
    tx: &mut sqlx::PgConnection,
    id: &str,
    control: Option<Permission>,
    add: Option<Permission>,
) -> Result<bool, Error> {
    let updated = sqlx::query(
        "UPDATE watch_rooms SET control_permission = COALESCE($2, control_permission), \
            add_permission = COALESCE($3, add_permission) \
         WHERE id = $1 AND (control_permission IS DISTINCT FROM COALESCE($2, control_permission) \
            OR add_permission IS DISTINCT FROM COALESCE($3, add_permission))",
    )
    .bind(id)
    .bind(control.map(Permission::as_str))
    .bind(add.map(Permission::as_str))
    .execute(&mut *tx)
    .await?;
    Ok(updated.rows_affected() > 0)
}

/// How close to its end a video must be for anyone to report it as ended.
const END_SLACK_MS: i64 = 5000;

/// Whether the current entry `from` has played to its end, as far as the room knows.
async fn at_end(
    tx: &mut sqlx::PgConnection,
    locked: &Locked,
    from: i64,
    now: OffsetDateTime,
) -> Result<bool, Error> {
    let duration: Option<Option<i32>> = sqlx::query_scalar(
        "SELECT v.duration_seconds FROM watch_room_queue q JOIN videos v ON v.id = q.video_id \
         WHERE q.id = $1",
    )
    .bind(from)
    .fetch_optional(&mut *tx)
    .await?;
    // Without a known duration, the report is trusted.
    let Some(Some(duration)) = duration else {
        return Ok(true);
    };
    let elapsed = if locked.playing {
        i64::try_from((now - locked.position_at).whole_milliseconds()).unwrap_or(0)
    } else {
        0
    };
    Ok(locked.position_ms + elapsed.max(0) >= i64::from(duration) * 1000 - END_SLACK_MS)
}

/// Whether the room's permissions let the participant with key `actor` apply `op`. The owner
/// may do anything but pick for autoplay, which only the server does.
async fn allowed(
    tx: &mut sqlx::PgConnection,
    id: &str,
    locked: &Locked,
    op: &RoomOp,
    actor: &str,
    now: OffsetDateTime,
) -> Result<bool, Error> {
    if matches!(op, RoomOp::Pick { .. }) {
        return Ok(false);
    }
    if actor == format!("u:{}", locked.owner_id) {
        return Ok(true);
    }
    let control = locked.control_permission == Permission::Everyone;
    let add = locked.add_permission == Permission::Everyone;
    Ok(match op {
        RoomOp::Kick { .. } | RoomOp::SetPermissions { .. } | RoomOp::Pick { .. } => false,
        // Playing a video now also changes what plays.
        RoomOp::Add { play: true, .. } => add && control,
        RoomOp::Add { .. } | RoomOp::AddMany { .. } => add,
        RoomOp::Remove { entry } => {
            control
                || (locked.current != Some(*entry)
                    && sqlx::query_scalar::<_, bool>(
                        "SELECT EXISTS (SELECT 1 FROM watch_room_queue \
                            WHERE id = $1 AND room_id = $2 AND added_key = $3)",
                    )
                    .bind(entry)
                    .bind(id)
                    .bind(actor)
                    .fetch_one(&mut *tx)
                    .await?)
        }
        // Everyone's player reports the end of a video, but only there.
        RoomOp::Advance { from } => {
            control || (locked.current == Some(*from) && at_end(tx, locked, *from, now).await?)
        }
        RoomOp::Play { .. }
        | RoomOp::Pause { .. }
        | RoomOp::Seek { .. }
        | RoomOp::Select { .. }
        | RoomOp::Move { .. }
        | RoomOp::Shuffle
        | RoomOp::SetAutoplay { .. } => control,
    })
}

fn changed(applied: bool) -> OpOutcome {
    if applied {
        OpOutcome::Applied
    } else {
        OpOutcome::Unchanged
    }
}

async fn apply_op(
    tx: &mut sqlx::PgConnection,
    id: &str,
    locked: &Locked,
    op: &RoomOp,
    actor: Option<&str>,
    now: OffsetDateTime,
) -> Result<OpOutcome, Error> {
    let current = locked.current;
    Ok(match op {
        RoomOp::Play { position_ms }
        | RoomOp::Pause { position_ms }
        | RoomOp::Seek { position_ms } => {
            if current.is_none() {
                return Ok(OpOutcome::Unchanged);
            }
            let playing = match op {
                RoomOp::Play { .. } => Some(true),
                RoomOp::Pause { .. } => Some(false),
                _ => None,
            };
            set_position(tx, id, playing, *position_ms, now).await?;
            OpOutcome::Applied
        }
        RoomOp::Advance { from } => {
            if current != Some(*from) {
                return Ok(OpOutcome::Unchanged);
            }
            advance(tx, id, locked, *from, now).await?;
            OpOutcome::Applied
        }
        RoomOp::Select { entry } => changed(select(tx, id, current, *entry, now).await?),
        RoomOp::Add {
            video_id,
            added_by,
            max_queue,
            play,
        } => {
            add(
                tx,
                id,
                current,
                (video_id, added_by, *max_queue, *play),
                actor,
                now,
            )
            .await?;
            OpOutcome::Applied
        }
        RoomOp::AddMany {
            video_ids,
            added_by,
            max_queue,
        } => match add_many(
            tx,
            id,
            current,
            (video_ids, added_by, *max_queue),
            actor,
            now,
        )
        .await?
        {
            0 => OpOutcome::Unchanged,
            added => OpOutcome::Added(added),
        },
        RoomOp::Remove { entry } => {
            if current == Some(*entry) {
                advance(tx, id, locked, *entry, now).await?;
                return Ok(OpOutcome::Applied);
            }
            let deleted = sqlx::query(
                "DELETE FROM watch_room_queue WHERE id = $1 AND room_id = $2 \
                     AND played IS NULL",
            )
            .bind(entry)
            .bind(id)
            .execute(&mut *tx)
            .await?;
            changed(deleted.rows_affected() > 0)
        }
        RoomOp::Move {
            entry,
            to,
            revision,
        } => {
            if *revision != locked.revision {
                return Ok(OpOutcome::Stale);
            }
            changed(move_entry(tx, id, current, *entry, *to).await?)
        }
        RoomOp::Shuffle => changed(shuffle(tx, id, current).await?),
        RoomOp::Kick { participant } => changed(kick(tx, id, *participant).await?),
        RoomOp::SetAutoplay { enabled } => changed(set_autoplay(tx, id, *enabled).await?),
        RoomOp::SetPermissions { control, add } => {
            changed(set_permissions(tx, id, *control, *add).await?)
        }
        RoomOp::Pick { source, candidates } => {
            changed(pick(tx, id, locked, *source, candidates).await?)
        }
    })
}

#[async_trait::async_trait]
impl RoomRepository for PgRoomRepository {
    async fn create(
        &self,
        owner_id: Uuid,
        id: &str,
        video: Option<(&str, &str)>,
        max_rooms: u32,
    ) -> Result<Room, Error> {
        let mut tx = self.pool.begin().await?;
        // Serializes concurrent creations of one user, so the limit holds.
        sqlx::query("SELECT 1 FROM users WHERE id = $1 FOR NO KEY UPDATE")
            .bind(owner_id)
            .execute(&mut *tx)
            .await?;
        let count: i64 = sqlx::query_scalar("SELECT count(*) FROM watch_rooms WHERE owner_id = $1")
            .bind(owner_id)
            .fetch_one(&mut *tx)
            .await?;
        if count >= i64::from(max_rooms) {
            return Err(Error::LimitReached);
        }
        sqlx::query("INSERT INTO watch_rooms (id, owner_id) VALUES ($1, $2)")
            .bind(id)
            .bind(owner_id)
            .execute(&mut *tx)
            .await?;
        if let Some((video_id, added_by)) = video {
            sqlx::query(
                "WITH e AS (INSERT INTO watch_room_queue (room_id, video_id, added_by) \
                    VALUES ($1, $2, $3) RETURNING id) \
                 UPDATE watch_rooms SET current_entry_id = (SELECT id FROM e) WHERE id = $1",
            )
            .bind(id)
            .bind(video_id)
            .bind(added_by)
            .execute(&mut *tx)
            .await?;
        }
        let room = sqlx::query_as::<_, Room>(ROOM_SELECT)
            .bind(id)
            .fetch_one(&mut *tx)
            .await?;
        tx.commit().await?;
        Ok(room)
    }

    async fn get(&self, id: &str) -> Result<Option<Room>, Error> {
        Ok(sqlx::query_as::<_, Room>(ROOM_SELECT)
            .bind(id)
            .fetch_optional(&self.pool)
            .await?)
    }

    async fn snapshot(&self, id: &str) -> Result<Option<RoomSnapshot>, Error> {
        let mut tx = self.pool.begin().await?;
        sqlx::query("SET TRANSACTION ISOLATION LEVEL REPEATABLE READ, READ ONLY")
            .execute(&mut *tx)
            .await?;
        let Some(room) = sqlx::query_as::<_, Room>(ROOM_SELECT)
            .bind(id)
            .fetch_optional(&mut *tx)
            .await?
        else {
            return Ok(None);
        };
        let queue = sqlx::query_as::<_, RoomEntry>(sqlx::AssertSqlSafe(format!(
            "{ENTRY_SELECT} WHERE q.room_id = $1 AND q.played IS NULL \
             ORDER BY (q.id = $2) IS TRUE DESC, q.position, q.id"
        )))
        .bind(id)
        .bind(room.current_entry_id)
        .fetch_all(&mut *tx)
        .await?;
        let history = sqlx::query_as::<_, RoomEntry>(sqlx::AssertSqlSafe(format!(
            "{ENTRY_SELECT} WHERE q.room_id = $1 AND q.played IS NOT NULL \
             ORDER BY q.played DESC LIMIT $2"
        )))
        .bind(id)
        .bind(MAX_HISTORY)
        .fetch_all(&mut *tx)
        .await?;
        let participants = sqlx::query_as::<_, Participant>(
            "SELECT id, key, user_id, display_name, joined_at, sync, place, status_revision \
             FROM watch_room_participants \
             WHERE room_id = $1 ORDER BY joined_at, id",
        )
        .bind(id)
        .fetch_all(&mut *tx)
        .await?;
        let pending = room.autoplay
            && room.current_entry_id.is_some()
            && room.autoplay_source == room.current_entry_id
            && queue.len() == 1;
        let up_next =
            match room.autoplay_video_id.as_deref() {
                Some(video_id) if pending => sqlx::query_as::<_, Video>(
                    "SELECT id, title, channel_id, channel_name, duration_seconds, published_at, \
                        thumbnail_url, updated_at FROM videos WHERE id = $1",
                )
                .bind(video_id)
                .fetch_optional(&mut *tx)
                .await?,
                _ => None,
            };
        tx.commit().await?;
        Ok(Some(RoomSnapshot {
            room,
            queue,
            history,
            participants,
            up_next,
        }))
    }

    async fn list_owned(&self, owner_id: Uuid) -> Result<Vec<RoomSummary>, Error> {
        Ok(sqlx::query_as::<_, RoomSummary>(
            "SELECT r.id, r.created_at, r.active_at, \
                (SELECT count(*) FROM watch_room_participants p WHERE p.room_id = r.id) \
                    AS participant_count, \
                (SELECT count(*) FROM watch_room_queue q \
                    WHERE q.room_id = r.id AND q.played IS NULL) AS queue_count, \
                v.id AS current_video_id, v.title AS current_title \
             FROM watch_rooms r \
             LEFT JOIN watch_room_queue q ON q.id = r.current_entry_id \
             LEFT JOIN videos v ON v.id = q.video_id \
             WHERE r.owner_id = $1 ORDER BY r.created_at DESC",
        )
        .bind(owner_id)
        .fetch_all(&self.pool)
        .await?)
    }

    async fn delete(&self, owner_id: Uuid, id: &str) -> Result<bool, Error> {
        let result = sqlx::query("DELETE FROM watch_rooms WHERE id = $1 AND owner_id = $2")
            .bind(id)
            .bind(owner_id)
            .execute(&self.pool)
            .await?;
        Ok(result.rows_affected() > 0)
    }

    async fn join(
        &self,
        room_id: &str,
        participant: &NewParticipant,
        max_participants: u32,
    ) -> Result<JoinOutcome, Error> {
        let mut tx = self.pool.begin().await?;
        let banned: Option<bool> = sqlx::query_scalar(
            "SELECT $2 = ANY (banned) FROM watch_rooms WHERE id = $1 FOR UPDATE",
        )
        .bind(room_id)
        .bind(&participant.key)
        .fetch_optional(&mut *tx)
        .await?;
        match banned {
            None => return Ok(JoinOutcome::NotFound),
            Some(true) => return Ok(JoinOutcome::Banned),
            Some(false) => {}
        }
        let count: i64 =
            sqlx::query_scalar("SELECT count(*) FROM watch_room_participants WHERE room_id = $1")
                .bind(room_id)
                .fetch_one(&mut *tx)
                .await?;
        if count >= i64::from(max_participants) {
            return Ok(JoinOutcome::Full);
        }
        sqlx::query(
            "INSERT INTO watch_room_participants (id, room_id, key, user_id, display_name) \
             VALUES ($1, $2, $3, $4, $5)",
        )
        .bind(participant.id)
        .bind(room_id)
        .bind(&participant.key)
        .bind(participant.user_id)
        .bind(&participant.display_name)
        .execute(&mut *tx)
        .await?;
        bump(&mut tx, room_id).await?;
        tx.commit().await?;
        Ok(JoinOutcome::Joined)
    }

    async fn leave(&self, participant_id: Uuid) -> Result<(), Error> {
        sqlx::query(
            "WITH gone AS (DELETE FROM watch_room_participants WHERE id = $1 RETURNING room_id) \
             UPDATE watch_rooms SET revision = revision + 1, active_at = now() \
             WHERE id IN (SELECT room_id FROM gone)",
        )
        .bind(participant_id)
        .execute(&self.pool)
        .await?;
        Ok(())
    }

    async fn set_status(
        &self,
        participant_id: Uuid,
        sync: Option<SyncState>,
        place: Option<Place>,
    ) -> Result<(), Error> {
        sqlx::query(
            "UPDATE watch_room_participants \
             SET sync = $2, place = $3, status_revision = status_revision + 1 \
             WHERE id = $1 AND (sync IS DISTINCT FROM $2 OR place IS DISTINCT FROM $3)",
        )
        .bind(participant_id)
        .bind(sync.map(SyncState::as_str))
        .bind(place.map(Place::as_str))
        .execute(&self.pool)
        .await?;
        Ok(())
    }

    async fn heartbeat(&self, participants: &[Uuid], rooms: &[String]) -> Result<(), Error> {
        let mut tx = self.pool.begin().await?;
        sqlx::query("UPDATE watch_room_participants SET seen_at = now() WHERE id = ANY($1)")
            .bind(participants)
            .execute(&mut *tx)
            .await?;
        sqlx::query("UPDATE watch_rooms SET active_at = now() WHERE id = ANY($1)")
            .bind(rooms)
            .execute(&mut *tx)
            .await?;
        tx.commit().await?;
        Ok(())
    }

    async fn apply_as(
        &self,
        room_id: &str,
        op: &RoomOp,
        actor: Option<&str>,
        now: OffsetDateTime,
    ) -> Result<OpOutcome, Error> {
        let mut tx = self.pool.begin().await?;
        let Some(locked) = lock_room(&mut tx, room_id).await? else {
            return Ok(OpOutcome::NotFound);
        };
        if let Some(actor) = actor
            && !allowed(&mut tx, room_id, &locked, op, actor, now).await?
        {
            return Ok(OpOutcome::Forbidden);
        }
        let outcome = apply_op(&mut tx, room_id, &locked, op, actor, now).await?;
        if matches!(outcome, OpOutcome::Applied | OpOutcome::Added(_)) {
            bump(&mut tx, room_id).await?;
            tx.commit().await?;
        }
        Ok(outcome)
    }

    async fn post_message(
        &self,
        room_id: &str,
        author_key: &str,
        name: &str,
        body: &str,
    ) -> Result<Option<ChatMessage>, Error> {
        let mut tx = self.pool.begin().await?;
        let message = sqlx::query_as::<_, ChatMessage>(
            "INSERT INTO watch_room_messages (room_id, author_key, name, body) \
             SELECT id, $2, $3, $4 FROM watch_rooms WHERE id = $1 \
             RETURNING id, room_id, author_key, name, body, sent_at",
        )
        .bind(room_id)
        .bind(author_key)
        .bind(name)
        .bind(body)
        .fetch_optional(&mut *tx)
        .await?;
        if message.is_some() {
            sqlx::query(
                "DELETE FROM watch_room_messages WHERE id IN (SELECT id FROM watch_room_messages \
                    WHERE room_id = $1 ORDER BY id DESC OFFSET $2)",
            )
            .bind(room_id)
            .bind(MAX_MESSAGES)
            .execute(&mut *tx)
            .await?;
        }
        tx.commit().await?;
        Ok(message)
    }

    async fn messages(&self, room_id: &str) -> Result<Vec<ChatMessage>, Error> {
        let mut messages = sqlx::query_as::<_, ChatMessage>(
            "SELECT id, room_id, author_key, name, body, sent_at FROM watch_room_messages \
             WHERE room_id = $1 ORDER BY id DESC LIMIT $2",
        )
        .bind(room_id)
        .bind(MAX_MESSAGES)
        .fetch_all(&self.pool)
        .await?;
        messages.reverse();
        Ok(messages)
    }

    async fn react(
        &self,
        room_id: &str,
        participant_id: Uuid,
        reaction: Reaction,
    ) -> Result<(), Error> {
        sqlx::query("SELECT pg_notify($1, $2)")
            .bind(REACTION_CHANNEL)
            .bind(format!("{room_id} {participant_id} {}", reaction.as_str()))
            .execute(&self.pool)
            .await?;
        Ok(())
    }

    async fn cleanup(&self, stale_after: Duration, expire_after: Duration) -> Result<u64, Error> {
        sqlx::query(
            "WITH gone AS (DELETE FROM watch_room_participants \
                WHERE seen_at < now() - make_interval(secs => $1) RETURNING room_id) \
             UPDATE watch_rooms SET revision = revision + 1 \
             WHERE id IN (SELECT room_id FROM gone)",
        )
        .bind(stale_after.as_secs_f64())
        .execute(&self.pool)
        .await?;
        let deleted = sqlx::query(
            "DELETE FROM watch_rooms r WHERE active_at < now() - make_interval(secs => $1) \
             AND NOT EXISTS (SELECT 1 FROM watch_room_participants p WHERE p.room_id = r.id)",
        )
        .bind(expire_after.as_secs_f64())
        .execute(&self.pool)
        .await?;
        Ok(deleted.rows_affected())
    }
}

/// What a [`RoomListener`] reports.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum RoomEvent {
    /// The room changed or was deleted.
    Changed(String),
    /// A participant of the room reported a new status.
    Status {
        room: String,
        participant: Uuid,
        status: ParticipantStatus,
    },
    Chat(ChatMessage),
    Reaction {
        room: String,
        participant: Uuid,
        reaction: Reaction,
    },
    /// The connection was lost and re-established; notifications may have been missed.
    Reconnected,
}

/// Receives room change notifications from all replicas.
pub struct RoomListener {
    listener: PgListener,
}

impl RoomListener {
    pub async fn connect(pool: &PgPool) -> Result<Self, Error> {
        let mut listener = PgListener::connect_with(pool).await?;
        listener
            .listen_all([ROOM_CHANNEL, STATUS_CHANNEL, CHAT_CHANNEL, REACTION_CHANNEL])
            .await?;
        Ok(Self { listener })
    }

    pub async fn recv(&mut self) -> Result<RoomEvent, Error> {
        loop {
            let Some(notification) = self.listener.try_recv().await? else {
                return Ok(RoomEvent::Reconnected);
            };
            if let Some(event) = event(&notification) {
                return Ok(event);
            }
        }
    }

    /// An event already received, without waiting.
    pub fn next_buffered(&mut self) -> Option<RoomEvent> {
        while let Some(notification) = self.listener.next_buffered() {
            if let Some(event) = event(&notification) {
                return Some(event);
            }
        }
        None
    }
}

fn event(notification: &PgNotification) -> Option<RoomEvent> {
    let payload = notification.payload();
    match notification.channel() {
        STATUS_CHANNEL => parse_status(payload),
        CHAT_CHANNEL => parse_chat(payload).map(RoomEvent::Chat),
        REACTION_CHANNEL => parse_reaction(payload),
        _ => Some(RoomEvent::Changed(payload.to_owned())),
    }
}

/// Parses `<room> <id> <sent ms> <author key>\n<name>\n<body>`.
fn parse_chat(payload: &str) -> Option<ChatMessage> {
    let mut lines = payload.splitn(3, '\n');
    let mut head = lines.next()?.split(' ');
    let name = lines.next()?.to_owned();
    let body = lines.next()?.to_owned();
    let room_id = head.next()?.to_owned();
    let id = head.next()?.parse().ok()?;
    let sent_ms: i64 = head.next()?.parse().ok()?;
    let author_key = head.next()?.to_owned();
    if head.next().is_some() {
        return None;
    }
    let sent_at =
        OffsetDateTime::from_unix_timestamp_nanos(i128::from(sent_ms) * 1_000_000).ok()?;
    Some(ChatMessage {
        id,
        room_id,
        author_key,
        name,
        body,
        sent_at,
    })
}

/// Parses `<room> <participant> <reaction>`.
fn parse_reaction(payload: &str) -> Option<RoomEvent> {
    let mut parts = payload.split(' ');
    let room = parts.next()?.to_owned();
    let participant = parts.next()?.parse().ok()?;
    let reaction = parts.next()?.parse().ok()?;
    if parts.next().is_some() {
        return None;
    }
    Some(RoomEvent::Reaction {
        room,
        participant,
        reaction,
    })
}

/// Parses `<room> <participant> <revision> <sync or -> <place or ->`.
fn parse_status(payload: &str) -> Option<RoomEvent> {
    fn optional<T: FromStr>(part: &str) -> Result<Option<T>, T::Err> {
        (part != "-").then(|| part.parse()).transpose()
    }
    let mut parts = payload.split(' ');
    let room = parts.next()?.to_owned();
    let participant = parts.next()?.parse().ok()?;
    let revision = parts.next()?.parse().ok()?;
    let sync = optional(parts.next()?).ok()?;
    let place = optional(parts.next()?).ok()?;
    if parts.next().is_some() {
        return None;
    }
    Some(RoomEvent::Status {
        room,
        participant,
        status: ParticipantStatus {
            sync,
            place,
            revision,
        },
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn status_payloads_parse() {
        let id = Uuid::from_u128(7);
        assert_eq!(
            parse_status(&format!("room {id} 3 behind shorts")),
            Some(RoomEvent::Status {
                room: "room".into(),
                participant: id,
                status: ParticipantStatus {
                    sync: Some(SyncState::Behind),
                    place: Some(Place::Shorts),
                    revision: 3,
                },
            })
        );
        assert_eq!(
            parse_status(&format!("room {id} 1 - watching")),
            Some(RoomEvent::Status {
                room: "room".into(),
                participant: id,
                status: ParticipantStatus {
                    sync: None,
                    place: Some(Place::Watching),
                    revision: 1,
                },
            })
        );
        assert_eq!(parse_status(&format!("room {id} 1 lost watching")), None);
        assert_eq!(parse_status(&format!("room {id} 1 - - extra")), None);
        assert_eq!(parse_status("room nope 1 - -"), None);
    }

    #[test]
    fn chat_payloads_parse() {
        let message = parse_chat("room 12 1700000000123 g:ab\nAda L.\nhi  there\nnot a line")
            .expect("parses");
        assert_eq!(message.room_id, "room");
        assert_eq!(message.id, 12);
        assert_eq!(message.author_key, "g:ab");
        assert_eq!(message.name, "Ada L.");
        assert_eq!(message.body, "hi  there\nnot a line");
        assert_eq!(
            message.sent_at.unix_timestamp_nanos(),
            1_700_000_000_123_000_000
        );
        assert_eq!(parse_chat("room 12 1 g:ab\nAda"), None);
        assert_eq!(parse_chat("room x 1 g:ab\nAda\nhi"), None);
        assert_eq!(parse_chat("room 12 1 g:ab extra\nAda\nhi"), None);
    }

    #[test]
    fn reaction_payloads_parse() {
        let id = Uuid::from_u128(7);
        assert_eq!(
            parse_reaction(&format!("room {id} clap")),
            Some(RoomEvent::Reaction {
                room: "room".into(),
                participant: id,
                reaction: Reaction::Clap,
            })
        );
        assert_eq!(parse_reaction(&format!("room {id} poop")), None);
        assert_eq!(parse_reaction(&format!("room {id} clap extra")), None);
    }
}
