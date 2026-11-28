//! Watch together rooms: the replica-local registry of open connections, kept in sync with
//! the database through `LISTEN`/`NOTIFY`.

use std::collections::{HashMap, HashSet};
use std::sync::{Arc, Mutex, PoisonError};
use std::time::{Duration, Instant};

use async_trait::async_trait;
use oxistream_db::{
    ParticipantStatus, PgPool, RoomEvent, RoomListener, RoomOp, RoomRepository, RoomSnapshot, Uuid,
    VideoUpsert,
};
use rustypipe::model as rp;
use rustypipe::param::Language;
use time::OffsetDateTime;
use tokio::sync::{broadcast, watch};

use crate::youtube::{self, YouTube};

pub const MAX_ROOMS_PER_USER: u32 = 10;
pub const MAX_PARTICIPANTS: u32 = 50;
pub const MAX_QUEUE: u32 = 100;
/// Rooms nobody was in for this long are deleted.
pub const EXPIRE_AFTER: Duration = Duration::from_hours(24);
const HEARTBEAT_INTERVAL: Duration = Duration::from_secs(30);
/// Participants of a replica that stopped sending heartbeats are dropped after this.
const STALE_AFTER: Duration = Duration::from_mins(2);
const CLEANUP_INTERVAL: Duration = Duration::from_mins(5);
const MAX_LISTEN_BACKOFF: Duration = Duration::from_mins(1);
/// A failed autoplay pick is tried again after this.
const PICK_RETRY: Duration = Duration::from_mins(1);
const MAX_PICK_CANDIDATES: usize = 20;
/// Status changes a slow connection may fall behind by; it skips older ones.
const STATUS_BUFFER: usize = 256;

/// Videos autoplay may follow a video with, best first.
#[async_trait]
pub trait RelatedSource: Send + Sync {
    async fn related(&self, video_id: &str) -> youtube::Result<Vec<VideoUpsert>>;
}

#[async_trait]
impl RelatedSource for YouTube {
    async fn related(&self, video_id: &str) -> youtube::Result<Vec<VideoUpsert>> {
        let details = self.video_details(video_id, Language::En).await?;
        Ok(details
            .recommended
            .items
            .into_iter()
            .filter_map(playable)
            .take(MAX_PICK_CANDIDATES)
            .collect())
    }
}

/// Regular videos only: no livestreams, premieres or shorts.
fn playable(v: rp::VideoItem) -> Option<VideoUpsert> {
    if v.is_live || v.is_upcoming || v.is_short {
        return None;
    }
    let channel = v.channel?;
    Some(VideoUpsert {
        thumbnail_url: v
            .thumbnail
            .iter()
            .max_by_key(|t| t.width * t.height)
            .map(|t| t.url.clone()),
        duration_seconds: Some(i32::try_from(v.duration?).ok()?),
        published_at: v.publish_date,
        channel_id: channel.id,
        channel_name: channel.name,
        id: v.id,
        title: v.name,
    })
}

/// The latest known state of a room.
#[derive(Debug, Clone)]
pub enum Update {
    Loading,
    Open(Arc<RoomSnapshot>),
    Closed,
}

#[derive(Clone)]
pub struct Rooms {
    inner: Arc<Inner>,
}

struct Inner {
    repo: Arc<dyn RoomRepository>,
    related: Option<Arc<dyn RelatedSource>>,
    local: Mutex<HashMap<String, Local>>,
    /// Autoplay picks started recently, by room and source entry.
    picks: Mutex<HashMap<(String, i64), Instant>>,
}

struct Local {
    tx: watch::Sender<Update>,
    statuses: broadcast::Sender<StatusChange>,
    participants: HashSet<Uuid>,
}

/// A participant reported a new status. Snapshots carry statuses too; the higher
/// `status.revision` is the newer one.
#[derive(Debug, Clone, Copy)]
pub struct StatusChange {
    pub participant: Uuid,
    pub status: ParticipantStatus,
}

/// A local participant's view of the room and its members' statuses.
pub struct Subscription {
    pub state: watch::Receiver<Update>,
    pub statuses: broadcast::Receiver<StatusChange>,
}

impl Rooms {
    /// Without `related`, autoplay never finds anything to append.
    pub fn new(repo: Arc<dyn RoomRepository>, related: Option<Arc<dyn RelatedSource>>) -> Self {
        Self {
            inner: Arc::new(Inner {
                repo,
                related,
                local: Mutex::default(),
                picks: Mutex::default(),
            }),
        }
    }

    pub fn repo(&self) -> &dyn RoomRepository {
        self.inner.repo.as_ref()
    }

    fn local(&self) -> std::sync::MutexGuard<'_, HashMap<String, Local>> {
        self.inner
            .local
            .lock()
            .unwrap_or_else(PoisonError::into_inner)
    }

    /// Registers a participant connected to this replica and returns its view of the room.
    pub async fn subscribe(&self, room_id: &str, participant: Uuid) -> Subscription {
        let subscription = {
            let mut local = self.local();
            let entry = local.entry(room_id.to_owned()).or_insert_with(|| Local {
                tx: watch::Sender::new(Update::Loading),
                statuses: broadcast::Sender::new(STATUS_BUFFER),
                participants: HashSet::new(),
            });
            entry.participants.insert(participant);
            Subscription {
                state: entry.tx.subscribe(),
                statuses: entry.statuses.subscribe(),
            }
        };
        self.refresh(room_id).await;
        subscription
    }

    /// Passes a status on to the room's local participants.
    fn status(&self, room_id: &str, change: StatusChange) {
        if let Some(entry) = self.local().get(room_id) {
            let _ = entry.statuses.send(change);
        }
    }

    pub fn unsubscribe(&self, room_id: &str, participant: Uuid) {
        let mut local = self.local();
        if let Some(entry) = local.get_mut(room_id) {
            entry.participants.remove(&participant);
            if entry.participants.is_empty() {
                local.remove(room_id);
            }
        }
    }

    /// Reloads a room for its local participants, if it has any here.
    pub async fn refresh(&self, room_id: &str) {
        if !self.local().contains_key(room_id) {
            return;
        }
        let snapshot = match self.repo().snapshot(room_id).await {
            Ok(Some(snapshot)) => Update::Open(Arc::new(snapshot)),
            Ok(None) => Update::Closed,
            Err(err) => {
                tracing::warn!(error = ?err, "could not load watch room");
                return;
            }
        };
        {
            let local = self.local();
            let Some(entry) = local.get(room_id) else {
                return;
            };
            // Loads may finish out of order; an older revision never replaces a newer one.
            entry
                .tx
                .send_if_modified(|current| match (&*current, &snapshot) {
                    (Update::Open(old), Update::Open(new))
                        if new.room.revision < old.room.revision =>
                    {
                        false
                    }
                    (Update::Closed, _) => false,
                    _ => {
                        *current = snapshot.clone();
                        true
                    }
                });
        }
        if let Update::Open(snapshot) = &snapshot {
            self.pick(snapshot);
        }
    }

    /// Picks what autoplay appends after the last entry. Every replica with participants
    /// may try; the database keeps the first pick.
    fn pick(&self, snapshot: &RoomSnapshot) {
        let Some(related) = self.inner.related.clone() else {
            return;
        };
        let Some((source, video_id)) = snapshot.needs_pick() else {
            return;
        };
        let room_id = snapshot.room.id.clone();
        {
            let mut picks = self
                .inner
                .picks
                .lock()
                .unwrap_or_else(PoisonError::into_inner);
            let now = Instant::now();
            picks.retain(|_, at| now.duration_since(*at) < PICK_RETRY);
            if picks.contains_key(&(room_id.clone(), source)) {
                return;
            }
            picks.insert((room_id.clone(), source), now);
        }
        let video_id = video_id.to_owned();
        let rooms = self.clone();
        tokio::spawn(async move {
            match related.related(&video_id).await {
                Ok(candidates) => {
                    let op = RoomOp::Pick { source, candidates };
                    let now = OffsetDateTime::now_utc();
                    if let Err(err) = rooms.repo().apply(&room_id, &op, now).await {
                        tracing::warn!(error = ?err, "could not store autoplay pick");
                    }
                }
                Err(err) => {
                    tracing::warn!(%video_id, error = %err, "could not pick autoplay video");
                    tokio::time::sleep(PICK_RETRY).await;
                    rooms.refresh(&room_id).await;
                }
            }
        });
    }

    fn local_rooms(&self) -> Vec<String> {
        self.local().keys().cloned().collect()
    }

    /// Follows changes from all replicas and keeps participants and rooms alive.
    pub fn spawn(&self, pool: PgPool) {
        let rooms = self.clone();
        tokio::spawn(async move { rooms.listen(pool).await });
        let rooms = self.clone();
        tokio::spawn(async move { rooms.maintain().await });
    }

    async fn listen(&self, pool: PgPool) {
        let mut backoff = Duration::from_secs(1);
        loop {
            let mut listener = match RoomListener::connect(&pool).await {
                Ok(listener) => listener,
                Err(err) => {
                    tracing::warn!(error = ?err, "could not listen for watch room changes");
                    tokio::time::sleep(backoff).await;
                    backoff = (backoff * 2).min(MAX_LISTEN_BACKOFF);
                    continue;
                }
            };
            backoff = Duration::from_secs(1);
            // Changes made before listening started would be missed otherwise.
            self.refresh_all().await;
            loop {
                let mut next = match listener.recv().await {
                    Ok(event) => Some(event),
                    Err(err) => {
                        tracing::warn!(error = ?err, "lost watch room notifications");
                        break;
                    }
                };
                let mut changed = HashSet::new();
                let mut reconnected = false;
                while let Some(event) = next {
                    match event {
                        RoomEvent::Changed(room) => {
                            changed.insert(room);
                        }
                        RoomEvent::Status {
                            room,
                            participant,
                            status,
                        } => self.status(
                            &room,
                            StatusChange {
                                participant,
                                status,
                            },
                        ),
                        RoomEvent::Reconnected => reconnected = true,
                    }
                    next = listener.next_buffered();
                }
                if reconnected {
                    self.refresh_all().await;
                } else {
                    for room in changed {
                        self.refresh(&room).await;
                    }
                }
            }
        }
    }

    async fn refresh_all(&self) {
        for room in self.local_rooms() {
            self.refresh(&room).await;
        }
    }

    async fn maintain(&self) {
        let mut heartbeat = tokio::time::interval(HEARTBEAT_INTERVAL);
        let mut cleanup = tokio::time::interval(CLEANUP_INTERVAL);
        loop {
            tokio::select! {
                _ = heartbeat.tick() => {
                    let (participants, rooms): (Vec<Uuid>, Vec<String>) = {
                        let local = self.local();
                        (
                            local.values().flat_map(|l| l.participants.iter().copied()).collect(),
                            local.keys().cloned().collect(),
                        )
                    };
                    if rooms.is_empty() {
                        continue;
                    }
                    if let Err(err) = self.repo().heartbeat(&participants, &rooms).await {
                        tracing::warn!(error = ?err, "could not refresh watch room participants");
                    }
                }
                _ = cleanup.tick() => {
                    match self.repo().cleanup(STALE_AFTER, EXPIRE_AFTER).await {
                        Ok(0) => {}
                        Ok(count) => tracing::debug!(count, "deleted expired watch rooms"),
                        Err(err) => tracing::warn!(error = ?err, "could not clean up watch rooms"),
                    }
                }
            }
        }
    }
}
