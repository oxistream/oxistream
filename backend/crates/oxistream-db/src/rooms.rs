use std::time::Duration;

use sqlx::PgPool;
use sqlx::postgres::{PgListener, PgNotification};
use time::OffsetDateTime;
use uuid::Uuid;

use crate::{Error, Video};

/// Notifications carry the ID of a room that changed or was deleted.
pub const ROOM_CHANNEL: &str = "watch_rooms";

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
}

#[derive(Debug, Clone, PartialEq, Eq, sqlx::FromRow)]
pub struct RoomEntry {
    pub entry_id: i64,
    /// Display name of whoever queued it.
    pub added_by: String,
    pub added_at: OffsetDateTime,
    #[sqlx(flatten)]
    pub video: Video,
}

#[derive(Debug, Clone, PartialEq, Eq, sqlx::FromRow)]
pub struct Participant {
    pub id: Uuid,
    /// `u:<user id>` or `g:<hash of the guest key>`.
    pub key: String,
    pub user_id: Option<Uuid>,
    pub display_name: String,
    pub joined_at: OffsetDateTime,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct NewParticipant {
    pub id: Uuid,
    pub key: String,
    pub user_id: Option<Uuid>,
    pub display_name: String,
}

/// A room with its queue (current entry first among the others, then by insertion) and
/// participants (by join time).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RoomSnapshot {
    pub room: Room,
    pub queue: Vec<RoomEntry>,
    pub participants: Vec<Participant>,
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
    /// Plays a queued entry now; the current one is dropped.
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
    Remove {
        entry: i64,
    },
    /// Removes a participant's connections and bans their key.
    Kick {
        participant: Uuid,
    },
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum OpOutcome {
    Applied,
    /// Valid, but nothing to do, e.g. a stale [`RoomOp::Advance`].
    Unchanged,
    NotFound,
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

    /// Keeps participants and their rooms from being cleaned up.
    async fn heartbeat(&self, participants: &[Uuid], rooms: &[String]) -> Result<(), Error>;

    /// Fails with [`Error::LimitReached`] when adding to a full queue. `now` becomes the
    /// position's reference time.
    async fn apply(
        &self,
        room_id: &str,
        op: &RoomOp,
        now: OffsetDateTime,
    ) -> Result<OpOutcome, Error>;

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
        r.playing, r.position_ms, r.position_at, r.revision, r.banned, r.created_at, r.active_at \
    FROM watch_rooms r JOIN users u ON u.id = r.owner_id WHERE r.id = $1";

struct Locked {
    current: Option<i64>,
}

async fn lock_room(tx: &mut sqlx::PgConnection, id: &str) -> Result<Option<Locked>, Error> {
    let row: Option<(Option<i64>,)> =
        sqlx::query_as("SELECT current_entry_id FROM watch_rooms WHERE id = $1 FOR UPDATE")
            .bind(id)
            .fetch_optional(&mut *tx)
            .await?;
    Ok(row.map(|(current,)| Locked { current }))
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

/// Drops `from` and plays the oldest remaining entry, if any.
async fn advance(
    tx: &mut sqlx::PgConnection,
    id: &str,
    from: i64,
    now: OffsetDateTime,
) -> Result<(), Error> {
    sqlx::query("DELETE FROM watch_room_queue WHERE id = $1")
        .bind(from)
        .execute(&mut *tx)
        .await?;
    let next: Option<i64> =
        sqlx::query_scalar("SELECT min(id) FROM watch_room_queue WHERE room_id = $1")
            .bind(id)
            .fetch_one(&mut *tx)
            .await?;
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
        "SELECT EXISTS (SELECT 1 FROM watch_room_queue WHERE id = $1 AND room_id = $2)",
    )
    .bind(entry)
    .bind(id)
    .fetch_one(&mut *tx)
    .await?;
    if !exists || current == Some(entry) {
        return Ok(false);
    }
    if let Some(current) = current {
        sqlx::query("DELETE FROM watch_room_queue WHERE id = $1")
            .bind(current)
            .execute(&mut *tx)
            .await?;
    }
    set_current(tx, id, Some(entry), true, now).await?;
    Ok(true)
}

async fn add(
    tx: &mut sqlx::PgConnection,
    id: &str,
    current: Option<i64>,
    (video_id, added_by, max_queue, play): (&str, &str, u32, bool),
    now: OffsetDateTime,
) -> Result<(), Error> {
    let count: i64 = sqlx::query_scalar("SELECT count(*) FROM watch_room_queue WHERE room_id = $1")
        .bind(id)
        .fetch_one(&mut *tx)
        .await?;
    // Playing now drops the current entry, so the queue does not grow.
    let replaced = i64::from(play && current.is_some());
    if count - replaced >= i64::from(max_queue) {
        return Err(Error::LimitReached);
    }
    let entry: i64 = sqlx::query_scalar(
        "INSERT INTO watch_room_queue (room_id, video_id, added_by) VALUES ($1, $2, $3) \
         RETURNING id",
    )
    .bind(id)
    .bind(video_id)
    .bind(added_by)
    .fetch_one(&mut *tx)
    .await?;
    if play {
        if let Some(current) = current {
            sqlx::query("DELETE FROM watch_room_queue WHERE id = $1")
                .bind(current)
                .execute(&mut *tx)
                .await?;
        }
        set_current(tx, id, Some(entry), true, now).await?;
    } else if current.is_none() {
        set_current(tx, id, Some(entry), false, now).await?;
    }
    Ok(())
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

/// Returns whether anything changed.
async fn apply_op(
    tx: &mut sqlx::PgConnection,
    id: &str,
    current: Option<i64>,
    op: &RoomOp,
    now: OffsetDateTime,
) -> Result<bool, Error> {
    match op {
        RoomOp::Play { position_ms }
        | RoomOp::Pause { position_ms }
        | RoomOp::Seek { position_ms } => {
            if current.is_none() {
                return Ok(false);
            }
            let playing = match op {
                RoomOp::Play { .. } => Some(true),
                RoomOp::Pause { .. } => Some(false),
                _ => None,
            };
            set_position(tx, id, playing, *position_ms, now).await?;
            Ok(true)
        }
        RoomOp::Advance { from } => {
            if current != Some(*from) {
                return Ok(false);
            }
            advance(tx, id, *from, now).await?;
            Ok(true)
        }
        RoomOp::Select { entry } => select(tx, id, current, *entry, now).await,
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
                now,
            )
            .await?;
            Ok(true)
        }
        RoomOp::Remove { entry } => {
            if current == Some(*entry) {
                advance(tx, id, *entry, now).await?;
                return Ok(true);
            }
            let deleted =
                sqlx::query("DELETE FROM watch_room_queue WHERE id = $1 AND room_id = $2")
                    .bind(entry)
                    .bind(id)
                    .execute(&mut *tx)
                    .await?;
            Ok(deleted.rows_affected() > 0)
        }
        RoomOp::Kick { participant } => kick(tx, id, *participant).await,
    }
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
        let queue = sqlx::query_as::<_, RoomEntry>(
            "SELECT q.id AS entry_id, q.added_by, q.added_at, v.id, v.title, v.channel_id, \
                v.channel_name, v.duration_seconds, v.published_at, v.thumbnail_url, v.updated_at \
             FROM watch_room_queue q JOIN videos v ON v.id = q.video_id \
             WHERE q.room_id = $1 ORDER BY (q.id = $2) IS TRUE DESC, q.id",
        )
        .bind(id)
        .bind(room.current_entry_id)
        .fetch_all(&mut *tx)
        .await?;
        let participants = sqlx::query_as::<_, Participant>(
            "SELECT id, key, user_id, display_name, joined_at FROM watch_room_participants \
             WHERE room_id = $1 ORDER BY joined_at, id",
        )
        .bind(id)
        .fetch_all(&mut *tx)
        .await?;
        tx.commit().await?;
        Ok(Some(RoomSnapshot {
            room,
            queue,
            participants,
        }))
    }

    async fn list_owned(&self, owner_id: Uuid) -> Result<Vec<RoomSummary>, Error> {
        Ok(sqlx::query_as::<_, RoomSummary>(
            "SELECT r.id, r.created_at, r.active_at, \
                (SELECT count(*) FROM watch_room_participants p WHERE p.room_id = r.id) \
                    AS participant_count, \
                (SELECT count(*) FROM watch_room_queue q WHERE q.room_id = r.id) AS queue_count, \
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

    async fn apply(
        &self,
        room_id: &str,
        op: &RoomOp,
        now: OffsetDateTime,
    ) -> Result<OpOutcome, Error> {
        let mut tx = self.pool.begin().await?;
        let Some(Locked { current }) = lock_room(&mut tx, room_id).await? else {
            return Ok(OpOutcome::NotFound);
        };
        let applied = apply_op(&mut tx, room_id, current, op, now).await?;
        if !applied {
            return Ok(OpOutcome::Unchanged);
        }
        bump(&mut tx, room_id).await?;
        tx.commit().await?;
        Ok(OpOutcome::Applied)
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
        listener.listen(ROOM_CHANNEL).await?;
        Ok(Self { listener })
    }

    pub async fn recv(&mut self) -> Result<RoomEvent, Error> {
        Ok(match self.listener.try_recv().await? {
            Some(notification) => event(&notification),
            None => RoomEvent::Reconnected,
        })
    }

    /// An event already received, without waiting.
    pub fn next_buffered(&mut self) -> Option<RoomEvent> {
        self.listener.next_buffered().map(|n| event(&n))
    }
}

fn event(notification: &PgNotification) -> RoomEvent {
    RoomEvent::Changed(notification.payload().to_owned())
}
