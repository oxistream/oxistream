//! Watch together rooms: the replica-local registry of open connections, kept in sync with
//! the database through `LISTEN`/`NOTIFY`.

use std::collections::{HashMap, HashSet};
use std::sync::{Arc, Mutex, PoisonError};
use std::time::Duration;

use oxistream_db::{PgPool, RoomEvent, RoomListener, RoomRepository, RoomSnapshot, Uuid};
use tokio::sync::watch;

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
    local: Mutex<HashMap<String, Local>>,
}

struct Local {
    tx: watch::Sender<Update>,
    participants: HashSet<Uuid>,
}

impl Rooms {
    pub fn new(repo: Arc<dyn RoomRepository>) -> Self {
        Self {
            inner: Arc::new(Inner {
                repo,
                local: Mutex::default(),
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
    pub async fn subscribe(&self, room_id: &str, participant: Uuid) -> watch::Receiver<Update> {
        let rx = {
            let mut local = self.local();
            let entry = local.entry(room_id.to_owned()).or_insert_with(|| Local {
                tx: watch::Sender::new(Update::Loading),
                participants: HashSet::new(),
            });
            entry.participants.insert(participant);
            entry.tx.subscribe()
        };
        self.refresh(room_id).await;
        rx
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
        let local = self.local();
        let Some(entry) = local.get(room_id) else {
            return;
        };
        // Loads may finish out of order; an older revision never replaces a newer one.
        entry
            .tx
            .send_if_modified(|current| match (&*current, &snapshot) {
                (Update::Open(old), Update::Open(new)) if new.room.revision < old.room.revision => {
                    false
                }
                (Update::Closed, _) => false,
                _ => {
                    *current = snapshot.clone();
                    true
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
                match listener.recv().await {
                    Ok(RoomEvent::Changed(room)) => {
                        let mut changed = HashSet::from([room]);
                        while let Some(RoomEvent::Changed(room)) = listener.next_buffered() {
                            changed.insert(room);
                        }
                        for room in changed {
                            self.refresh(&room).await;
                        }
                    }
                    Ok(RoomEvent::Reconnected) => self.refresh_all().await,
                    Err(err) => {
                        tracing::warn!(error = ?err, "lost watch room notifications");
                        break;
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
