use std::fmt::Write as _;
use std::sync::Arc;
use std::time::Duration;

use axum::extract::ws::{CloseFrame, Message, Utf8Bytes, WebSocket, WebSocketUpgrade, close_code};
use axum::extract::{Path, State};
use axum::http::{HeaderMap, Uri};
use axum::response::{IntoResponse, Response};
use oxistream_db::{JoinOutcome, NewParticipant, OpOutcome, RoomOp, RoomSnapshot, User, Uuid};
use rustypipe::param::Language;
use rustypipe::validate;
use sha2::{Digest, Sha256};
use time::OffsetDateTime;
use tokio::time::Instant;

use super::protocol::{
    RoomClientMessage, RoomErrorCode, RoomServerMessage, history, room_state, unix_ms,
};
use super::{check_room_id, ensure_cached};
use crate::AppState;
use crate::api::error::ApiError;
use crate::api::lang::ContentLanguage;
use crate::auth::{OptionalUser, csrf};
use crate::rooms::{MAX_PARTICIPANTS, MAX_QUEUE, Update};

const MAX_MESSAGE_BYTES: usize = 4096;
const JOIN_TIMEOUT: Duration = Duration::from_secs(10);
const KEEPALIVE_INTERVAL: Duration = Duration::from_secs(30);
const MAX_NAME_CHARS: usize = 32;
/// How far back a client may date a play or seek, to account for its latency.
const MAX_BACKDATE: Duration = Duration::from_secs(5);
/// Positions beyond a day are not valid for any video we can play.
const MAX_POSITION_SECONDS: f64 = 86_400.0;

/// Upgrades to the room WebSocket. Browsers send `Origin` on WebSocket handshakes but
/// cookies are not bound to it, so it is checked like on state-changing requests.
pub async fn connect(
    State(state): State<AppState>,
    OptionalUser(user): OptionalUser,
    ContentLanguage(lang): ContentLanguage,
    Path(id): Path<String>,
    headers: HeaderMap,
    uri: Uri,
    ws: WebSocketUpgrade,
) -> Response {
    let public_origin = state.auth.config().public_origin.as_deref();
    if !csrf::origin_allowed(&headers, &uri, public_origin) {
        return ApiError::origin_rejected().into_response();
    }
    if let Err(err) = check_room_id(&id) {
        return err.into_response();
    }
    ws.max_message_size(MAX_MESSAGE_BYTES)
        .max_frame_size(MAX_MESSAGE_BYTES)
        .on_upgrade(move |socket| async move {
            let mut conn = Connection {
                state,
                room_id: id,
                user,
                lang,
                socket,
                limits: Limits::new(),
                history_sent: None,
            };
            conn.run().await;
        })
}

/// Token buckets per connection: one for all messages, a stricter one for adding videos,
/// which may ask YouTube. A batch takes one, plus one per video not cached yet.
struct Limits {
    messages: Bucket,
    adds: Bucket,
}

impl Limits {
    fn new() -> Self {
        Self {
            messages: Bucket::new(20.0, 10.0),
            adds: Bucket::new(5.0, 0.5),
        }
    }
}

struct Bucket {
    tokens: f64,
    burst: f64,
    per_second: f64,
    last: Instant,
}

impl Bucket {
    fn new(burst: f64, per_second: f64) -> Self {
        Self {
            tokens: burst,
            burst,
            per_second,
            last: Instant::now(),
        }
    }

    fn take(&mut self) -> bool {
        let now = Instant::now();
        let elapsed = now.duration_since(self.last).as_secs_f64();
        self.last = now;
        self.tokens = (self.tokens + elapsed * self.per_second).min(self.burst);
        if self.tokens >= 1.0 {
            self.tokens -= 1.0;
            true
        } else {
            false
        }
    }
}

struct Identity {
    key: String,
    name: String,
}

/// Signed-in users are known by their account; guests by a key they keep.
fn identity(user: Option<&User>, name: Option<&str>, guest_key: Option<&str>) -> Option<Identity> {
    if let Some(user) = user {
        return Some(Identity {
            key: format!("u:{}", user.id),
            name: user.username.clone(),
        });
    }
    let name = name?.trim();
    let name_ok = !name.is_empty()
        && name.chars().count() <= MAX_NAME_CHARS
        && !name.chars().any(char::is_control);
    let key = guest_key?;
    let key_ok = (16..=128).contains(&key.len())
        && key
            .bytes()
            .all(|b| b.is_ascii_alphanumeric() || b == b'-' || b == b'_');
    if !name_ok || !key_ok {
        return None;
    }
    let hash = Sha256::digest(key.as_bytes());
    let hex = hash.iter().fold(String::new(), |mut hex, b| {
        let _ = write!(hex, "{b:02x}");
        hex
    });
    Some(Identity {
        key: format!("g:{hex}"),
        name: name.to_owned(),
    })
}

fn position_ms(seconds: f64) -> Option<i64> {
    #[allow(clippy::cast_possible_truncation)]
    (seconds.is_finite() && (0.0..=MAX_POSITION_SECONDS).contains(&seconds))
        .then(|| (seconds * 1000.0).round() as i64)
}

/// When a position was read: the client's estimate of the server clock, clamped to the recent
/// past, so latency does not put everyone behind whoever played or seeked.
fn position_time(at: Option<f64>, now: OffsetDateTime) -> OffsetDateTime {
    let Some(at) = at.filter(|at| at.is_finite()) else {
        return now;
    };
    // Whole milliseconds first: nanoseconds since the epoch exceed f64's exact range.
    let whole = at.floor();
    #[allow(clippy::cast_possible_truncation)]
    let nanos = i128::from(whole as i64) * 1_000_000 + ((at - whole) * 1_000_000.0) as i128;
    let at = OffsetDateTime::from_unix_timestamp_nanos(nanos).unwrap_or(now);
    at.clamp(now - MAX_BACKDATE, now)
}

fn now_ms() -> i64 {
    unix_ms(OffsetDateTime::now_utc())
}

fn new_participant_id() -> Uuid {
    let mut bytes = [0u8; 16];
    getrandom::fill(&mut bytes).expect("system random number generator");
    uuid::Builder::from_random_bytes(bytes).into_uuid()
}

struct Connection {
    state: AppState,
    room_id: String,
    user: Option<User>,
    lang: Language,
    socket: WebSocket,
    limits: Limits,
    /// The newest played entry of the history last sent (0 for none), once one was.
    history_sent: Option<i64>,
}

/// How the connection ends.
enum End {
    /// The client left or the connection broke.
    Gone,
    /// The owner removed the participant; it should not reconnect.
    Kicked,
    /// The room is gone; the client should not reconnect.
    Closed,
    /// The client may reconnect, e.g. after this replica lost track of it.
    Retry,
}

impl Connection {
    async fn send(&mut self, message: &RoomServerMessage) -> bool {
        let json = serde_json::to_string(message).expect("room messages serialize");
        self.socket.send(Message::Text(json.into())).await.is_ok()
    }

    async fn close(&mut self, code: u16, reason: &'static str) {
        let _ = self
            .socket
            .send(Message::Close(Some(CloseFrame {
                code,
                reason: Utf8Bytes::from_static(reason),
            })))
            .await;
    }

    async fn fail(&mut self, code: RoomErrorCode) {
        self.send(&RoomServerMessage::Error { code }).await;
        self.close(close_code::POLICY, "").await;
    }

    async fn run(&mut self) {
        let Ok(Some(Message::Text(text))) = tokio::time::timeout(JOIN_TIMEOUT, self.socket.recv())
            .await
            .map(|m| m.and_then(Result::ok))
        else {
            return self.close(close_code::POLICY, "join expected").await;
        };
        let Ok(RoomClientMessage::Join { name, guest_key }) = serde_json::from_str(&text) else {
            return self.fail(RoomErrorCode::Invalid).await;
        };
        let Some(identity) = identity(self.user.as_ref(), name.as_deref(), guest_key.as_deref())
        else {
            return self.fail(RoomErrorCode::Invalid).await;
        };

        let participant = new_participant_id();
        let joined = self
            .state
            .rooms
            .repo()
            .join(
                &self.room_id,
                &NewParticipant {
                    id: participant,
                    key: identity.key.clone(),
                    user_id: self.user.as_ref().map(|u| u.id),
                    display_name: identity.name.clone(),
                },
                MAX_PARTICIPANTS,
            )
            .await;
        match joined {
            Ok(JoinOutcome::Joined) => {}
            Ok(JoinOutcome::NotFound) => return self.fail(RoomErrorCode::NotFound).await,
            Ok(JoinOutcome::Full) => return self.fail(RoomErrorCode::RoomFull).await,
            Ok(JoinOutcome::Banned) => {
                self.send(&RoomServerMessage::Kicked).await;
                return self.close(close_code::POLICY, "").await;
            }
            Err(err) => {
                tracing::warn!(error = ?err, "could not join watch room");
                return self.fail(RoomErrorCode::Internal).await;
            }
        }

        let rooms = self.state.rooms.clone();
        let rx = rooms.subscribe(&self.room_id, participant).await;
        let end = self.session(participant, &identity, rx).await;
        rooms.unsubscribe(&self.room_id, participant);
        if let Err(err) = rooms.repo().leave(participant).await {
            tracing::warn!(error = ?err, "could not leave watch room");
        }
        match end {
            End::Gone => {}
            End::Kicked | End::Closed => {
                let message = if matches!(end, End::Kicked) {
                    RoomServerMessage::Kicked
                } else {
                    RoomServerMessage::Closed
                };
                self.send(&message).await;
                self.close(close_code::NORMAL, "").await;
            }
            End::Retry => self.close(close_code::RESTART, "").await,
        }
    }

    async fn session(
        &mut self,
        participant: Uuid,
        identity: &Identity,
        mut rx: tokio::sync::watch::Receiver<Update>,
    ) -> End {
        let owner_id = match &*rx.borrow_and_update() {
            Update::Open(snapshot) => snapshot.room.owner_id,
            Update::Closed => return End::Closed,
            Update::Loading => return End::Retry,
        };
        let is_owner = self.user.as_ref().is_some_and(|u| u.id == owner_id);
        let welcome = RoomServerMessage::Welcome {
            participant_id: participant,
            is_owner,
            server_time: now_ms(),
        };
        if !self.send(&welcome).await {
            return End::Gone;
        }
        // Marked as changed so the first state goes out right away.
        rx.mark_changed();
        let mut keepalive = tokio::time::interval(KEEPALIVE_INTERVAL);
        keepalive.reset();
        loop {
            tokio::select! {
                message = self.socket.recv() => match message {
                    Some(Ok(Message::Text(text))) => {
                        self.handle(&text, &identity.name, is_owner, owner_id).await;
                    }
                    Some(Ok(Message::Binary(_))) => {
                        self.send(&RoomServerMessage::Error { code: RoomErrorCode::Invalid }).await;
                    }
                    Some(Ok(Message::Ping(_) | Message::Pong(_))) => {}
                    Some(Ok(Message::Close(_)) | Err(_)) | None => return End::Gone,
                },
                changed = rx.changed() => {
                    if changed.is_err() {
                        return End::Retry;
                    }
                    let update = rx.borrow_and_update().clone();
                    match update {
                        Update::Loading => {}
                        Update::Closed => return End::Closed,
                        Update::Open(snapshot) => {
                            if let Some(end) = removed(&snapshot, participant, &identity.key) {
                                return end;
                            }
                            if !self.send_state(&snapshot).await {
                                return End::Gone;
                            }
                        }
                    }
                }
                _ = keepalive.tick() => {
                    if self.socket.send(Message::Ping(Vec::new().into())).await.is_err() {
                        return End::Gone;
                    }
                }
            }
        }
    }

    async fn send_state(&mut self, snapshot: &Arc<RoomSnapshot>) -> bool {
        // The history only changes when an entry is played, which then comes first.
        let newest = snapshot.history.first().and_then(|e| e.played).unwrap_or(0);
        if self.history_sent != Some(newest) {
            let entries = history(snapshot, &|url| self.state.image_url(url));
            if !self.send(&RoomServerMessage::History { entries }).await {
                return false;
            }
            self.history_sent = Some(newest);
        }
        let room = room_state(snapshot, &|url| self.state.image_url(url));
        self.send(&RoomServerMessage::State {
            room: Box::new(room),
        })
        .await
    }

    async fn handle(&mut self, text: &str, name: &str, is_owner: bool, owner_id: Uuid) {
        if !self.limits.messages.take() {
            self.send(&RoomServerMessage::Error {
                code: RoomErrorCode::RateLimited,
            })
            .await;
            return;
        }
        let Ok(message) = serde_json::from_str::<RoomClientMessage>(text) else {
            return self.reject(RoomErrorCode::Invalid).await;
        };
        let mut now = OffsetDateTime::now_utc();
        let op = match message {
            RoomClientMessage::Ping { t } => {
                let pong = RoomServerMessage::Pong {
                    t,
                    server_time: now_ms(),
                };
                self.send(&pong).await;
                return;
            }
            RoomClientMessage::Join { .. } => return self.reject(RoomErrorCode::Invalid).await,
            RoomClientMessage::Play { position, at } => match position_ms(position) {
                Some(position_ms) => {
                    now = position_time(at, now);
                    RoomOp::Play { position_ms }
                }
                None => return self.reject(RoomErrorCode::Invalid).await,
            },
            RoomClientMessage::Pause { position } => match position_ms(position) {
                Some(position_ms) => RoomOp::Pause { position_ms },
                None => return self.reject(RoomErrorCode::Invalid).await,
            },
            RoomClientMessage::Seek { position, at } => match position_ms(position) {
                Some(position_ms) => {
                    now = position_time(at, now);
                    RoomOp::Seek { position_ms }
                }
                None => return self.reject(RoomErrorCode::Invalid).await,
            },
            RoomClientMessage::Ended { entry } | RoomClientMessage::Skip { entry } => {
                RoomOp::Advance { from: entry }
            }
            RoomClientMessage::Select { entry } => RoomOp::Select { entry },
            RoomClientMessage::Remove { entry } => RoomOp::Remove { entry },
            RoomClientMessage::Move {
                entry,
                to,
                revision,
            } => RoomOp::Move {
                entry,
                to,
                revision,
            },
            RoomClientMessage::Shuffle => RoomOp::Shuffle,
            RoomClientMessage::Autoplay { enabled } => RoomOp::SetAutoplay { enabled },
            RoomClientMessage::AddMany { video_ids } => {
                return self.add_many(video_ids, name).await;
            }
            RoomClientMessage::Kick { participant } => {
                if !is_owner {
                    return self.reject(RoomErrorCode::Forbidden).await;
                }
                let target = self.state.rooms.repo().snapshot(&self.room_id).await;
                let target_is_owner = target.ok().flatten().is_some_and(|s| {
                    s.participants
                        .iter()
                        .any(|p| p.id == participant && p.user_id == Some(owner_id))
                });
                if target_is_owner {
                    return self.reject(RoomErrorCode::Invalid).await;
                }
                RoomOp::Kick { participant }
            }
            RoomClientMessage::Add { video_id, play } => {
                match self.add_op(video_id, play, name).await {
                    Ok(op) => op,
                    Err(code) => return self.reject(code).await,
                }
            }
        };
        let result = self.state.rooms.repo().apply(&self.room_id, &op, now).await;
        match result {
            Ok(OpOutcome::Stale) => self.reject(RoomErrorCode::Stale).await,
            Ok(_) => {}
            Err(oxistream_db::Error::LimitReached) => self.reject(RoomErrorCode::QueueFull).await,
            Err(err) => {
                tracing::warn!(error = ?err, "could not change watch room");
                self.reject(RoomErrorCode::Internal).await;
            }
        }
    }

    async fn add_op(
        &mut self,
        video_id: String,
        play: bool,
        name: &str,
    ) -> Result<RoomOp, RoomErrorCode> {
        if validate::video_id(&video_id).is_err() {
            return Err(RoomErrorCode::Invalid);
        }
        if !self.limits.adds.take() {
            return Err(RoomErrorCode::RateLimited);
        }
        if Box::pin(ensure_cached(&self.state, &video_id, self.lang))
            .await
            .is_err()
        {
            return Err(RoomErrorCode::VideoUnavailable);
        }
        Ok(RoomOp::Add {
            video_id,
            added_by: name.to_owned(),
            max_queue: MAX_QUEUE,
            play,
        })
    }

    /// Queues what is cached or can still be fetched within the rate limit, and tells the
    /// sender how many were skipped.
    async fn add_many(&mut self, video_ids: Vec<String>, name: &str) {
        let max = usize::try_from(MAX_QUEUE).unwrap_or(usize::MAX);
        let valid = video_ids.iter().all(|id| validate::video_id(id).is_ok());
        if video_ids.is_empty() || video_ids.len() > max || !valid {
            return self.reject(RoomErrorCode::Invalid).await;
        }
        if !self.limits.adds.take() {
            return self.reject(RoomErrorCode::RateLimited).await;
        }
        let requested = u32::try_from(video_ids.len()).unwrap_or(u32::MAX);
        let mut available = Vec::with_capacity(video_ids.len());
        for video_id in video_ids {
            let cached = matches!(self.state.videos.get(&video_id).await, Ok(Some(_)));
            let fetched = !cached
                && self.limits.adds.take()
                && Box::pin(ensure_cached(&self.state, &video_id, self.lang))
                    .await
                    .is_ok();
            if cached || fetched {
                available.push(video_id);
            }
        }
        let added = if available.is_empty() {
            0
        } else {
            let op = RoomOp::AddMany {
                video_ids: available,
                added_by: name.to_owned(),
                max_queue: MAX_QUEUE,
            };
            let now = OffsetDateTime::now_utc();
            match self.state.rooms.repo().apply(&self.room_id, &op, now).await {
                Ok(OpOutcome::Added(added)) => added,
                Ok(_) => 0,
                Err(oxistream_db::Error::LimitReached) => {
                    return self.reject(RoomErrorCode::QueueFull).await;
                }
                Err(err) => {
                    tracing::warn!(error = ?err, "could not change watch room");
                    return self.reject(RoomErrorCode::Internal).await;
                }
            }
        };
        self.send(&RoomServerMessage::Added {
            added,
            skipped: requested - added,
        })
        .await;
    }

    async fn reject(&mut self, code: RoomErrorCode) {
        self.send(&RoomServerMessage::Error { code }).await;
    }
}

/// Whether this participant is no longer in the room, and if so, why.
fn removed(snapshot: &RoomSnapshot, participant: Uuid, key: &str) -> Option<End> {
    if snapshot.participants.iter().any(|p| p.id == participant) {
        None
    } else if snapshot.room.banned.iter().any(|k| k == key) {
        Some(End::Kicked)
    } else {
        Some(End::Retry)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn guests_need_a_name_and_key() {
        let key = "abcdefghijklmnop";
        let guest = identity(None, Some("  Ada "), Some(key)).unwrap();
        assert_eq!(guest.name, "Ada");
        assert!(guest.key.starts_with("g:"));
        assert_ne!(guest.key, format!("g:{key}"));
        assert!(identity(None, Some(" "), Some(key)).is_none());
        assert!(identity(None, Some("a\u{7}"), Some(key)).is_none());
        assert!(identity(None, Some(&"x".repeat(33)), Some(key)).is_none());
        assert!(identity(None, Some("Ada"), Some("short")).is_none());
        assert!(identity(None, Some("Ada"), Some("abcdefghijklmno/")).is_none());
        assert!(identity(None, Some("Ada"), None).is_none());
    }

    #[test]
    fn positions_are_bounded() {
        assert_eq!(position_ms(12.3456), Some(12_346));
        assert_eq!(position_ms(0.0), Some(0));
        assert_eq!(position_ms(-1.0), None);
        assert_eq!(position_ms(f64::NAN), None);
        assert_eq!(position_ms(f64::INFINITY), None);
        assert_eq!(position_ms(90_000.0), None);
    }

    #[test]
    #[allow(clippy::cast_precision_loss)]
    fn positions_can_be_dated_back_a_little() {
        let now = OffsetDateTime::from_unix_timestamp(1_700_000_000).unwrap();
        let ms = |t: OffsetDateTime| unix_ms(t) as f64;
        assert_eq!(position_time(None, now), now);
        assert_eq!(position_time(Some(f64::NAN), now), now);
        let earlier = now - Duration::from_millis(80);
        assert_eq!(position_time(Some(ms(earlier)), now), earlier);
        assert_eq!(position_time(Some(ms(now) + 1000.0), now), now);
        assert_eq!(position_time(Some(0.0), now), now - MAX_BACKDATE);
    }

    #[test]
    fn buckets_refill_over_time() {
        let mut bucket = Bucket::new(2.0, 1.0);
        assert!(bucket.take());
        assert!(bucket.take());
        assert!(!bucket.take());
        bucket.last -= Duration::from_secs(1);
        assert!(bucket.take());
    }
}
