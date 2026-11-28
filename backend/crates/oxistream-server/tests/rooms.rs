//! Needs `OXISTREAM_TEST_DATABASE_URL` or a Docker daemon; run with `cargo test -- --include-ignored`.

mod common;

use std::sync::Arc;
use std::time::Duration;

use axum::Router;
use axum::body::Body;
use axum::http::{Method, Request, StatusCode, header};
use http_body_util::BodyExt;
use tower::ServiceExt;

use common::{isolated_test_pool, state_with_pool, unique_suffix};
use futures_util::{SinkExt, StreamExt};
use oxistream_db::{
    Error, JoinOutcome, NewParticipant, NewSession, NewUser, OpOutcome, PgPool, PgRoomRepository,
    PgSessionRepository, PgUserRepository, PgVideoRepository, RoomOp, RoomRepository,
    SessionRepository, User, UserRepository, Uuid, VideoRepository, VideoUpsert,
};
use oxistream_server::auth::cookie;
use oxistream_server::rooms::{RelatedSource, Rooms};
use oxistream_server::youtube;
use serde_json::{Value, json};
use time::OffsetDateTime;
use tokio::net::TcpStream;
use tokio_tungstenite::tungstenite::Message;
use tokio_tungstenite::tungstenite::client::IntoClientRequest;
use tokio_tungstenite::{MaybeTlsStream, WebSocketStream};

const VIDEOS: [&str; 6] = [
    "vvvvvvvvvv0",
    "vvvvvvvvvv4",
    "vvvvvvvvvv8",
    "vvvvvvvvvvA",
    "vvvvvvvvvvE",
    "vvvvvvvvvvI",
];
const ROOM: &str = "rrrrrrrrrrrrrrrrrrrrrA";

async fn user(pool: &PgPool) -> User {
    PgUserRepository::new(pool.clone())
        .create(&NewUser {
            username: format!("u_{}", unique_suffix()),
            password_hash: None,
        })
        .await
        .unwrap()
}

async fn session(pool: &PgPool, user: &User) -> String {
    let (token, token_hash) = cookie::new_token();
    PgSessionRepository::new(pool.clone())
        .create(&NewSession {
            token_hash,
            user_id: user.id,
            expires_at: OffsetDateTime::now_utc() + time::Duration::days(1),
            user_agent: None,
            ip: None,
            oidc_sid: None,
            oidc_id_token: None,
        })
        .await
        .unwrap();
    token
}

async fn cache_videos(pool: &PgPool) {
    let videos = PgVideoRepository::new(pool.clone());
    for id in VIDEOS {
        videos
            .upsert(&VideoUpsert {
                id: id.to_owned(),
                title: format!("Video {id}"),
                channel_id: "UCaaaaaaaaaaaaaaaaaaaaaa".into(),
                channel_name: "Alpha".into(),
                duration_seconds: Some(213),
                published_at: None,
                thumbnail_url: None,
            })
            .await
            .unwrap();
    }
}

fn guest(key: &str) -> NewParticipant {
    NewParticipant {
        id: Uuid::from_u128(rand_u128()),
        key: key.to_owned(),
        user_id: None,
        display_name: "Guest".into(),
    }
}

fn rand_u128() -> u128 {
    let mut bytes = [0u8; 16];
    getrandom::fill(&mut bytes).unwrap();
    u128::from_le_bytes(bytes)
}

fn add(video_id: &str) -> RoomOp {
    RoomOp::Add {
        video_id: video_id.to_owned(),
        added_by: "Ada".into(),
        max_queue: 3,
        play: false,
    }
}

async fn queue(repo: &PgRoomRepository) -> (Option<i64>, Vec<String>, bool) {
    let s = repo.snapshot(ROOM).await.unwrap().unwrap();
    (
        s.room.current_entry_id,
        s.queue.into_iter().map(|e| e.video.id).collect(),
        s.room.playing,
    )
}

#[tokio::test]
#[ignore = "requires Docker or OXISTREAM_TEST_DATABASE_URL"]
async fn queue_advances_once_and_respects_limits() {
    let db = isolated_test_pool().await;
    let repo = PgRoomRepository::new(db.pool.clone());
    let owner = user(&db.pool).await;
    cache_videos(&db.pool).await;
    let now = OffsetDateTime::now_utc();

    let room = repo
        .create(owner.id, ROOM, Some((VIDEOS[0], "Ada")), 1)
        .await
        .unwrap();
    assert_eq!(room.owner, owner.username);
    let first = room.current_entry_id.expect("starts with the video");
    assert!(matches!(
        repo.create(owner.id, "rrrrrrrrrrrrrrrrrrrrrB", None, 1)
            .await,
        Err(Error::LimitReached)
    ));

    repo.apply(ROOM, &add(VIDEOS[1]), now).await.unwrap();
    repo.apply(ROOM, &add(VIDEOS[2]), now).await.unwrap();
    assert!(matches!(
        repo.apply(ROOM, &add(VIDEOS[0]), now).await,
        Err(Error::LimitReached)
    ));
    assert_eq!(queue(&repo).await.1, &VIDEOS[..3]);

    let play = RoomOp::Play { position_ms: 5000 };
    assert_eq!(
        repo.apply(ROOM, &play, now).await.unwrap(),
        OpOutcome::Applied
    );
    let paused = RoomOp::Pause { position_ms: 7000 };
    repo.apply(ROOM, &paused, now).await.unwrap();
    let s = repo.snapshot(ROOM).await.unwrap().unwrap();
    assert!(!s.room.playing);
    assert_eq!(s.room.position_ms, 7000);

    // Two clients report the end of the same entry: the queue moves on once.
    let ended = RoomOp::Advance { from: first };
    assert_eq!(
        repo.apply(ROOM, &ended, now).await.unwrap(),
        OpOutcome::Applied
    );
    assert_eq!(
        repo.apply(ROOM, &ended, now).await.unwrap(),
        OpOutcome::Unchanged
    );
    let (current, ids, playing) = queue(&repo).await;
    assert_eq!(ids, &VIDEOS[1..3]);
    assert!(playing, "the next entry autoplays");

    let s = repo.snapshot(ROOM).await.unwrap().unwrap();
    let last = s.queue[1].entry_id;
    repo.apply(ROOM, &RoomOp::Select { entry: last }, now)
        .await
        .unwrap();
    assert_eq!(queue(&repo).await.1, [VIDEOS[2]]);
    repo.apply(ROOM, &RoomOp::Remove { entry: last }, now)
        .await
        .unwrap();
    let (current_after, ids, playing) = queue(&repo).await;
    assert!(current.is_some() && current_after.is_none() && ids.is_empty() && !playing);
    assert_eq!(
        repo.apply(ROOM, &play, now).await.unwrap(),
        OpOutcome::Unchanged,
        "nothing to play"
    );

    // A video added to an empty room becomes current, paused.
    repo.apply(ROOM, &add(VIDEOS[0]), now).await.unwrap();
    let (current, ids, playing) = queue(&repo).await;
    assert!(current.is_some() && !playing);
    assert_eq!(ids, [VIDEOS[0]]);

    // Playing a video now replaces the current entry, even with a full queue.
    repo.apply(ROOM, &add(VIDEOS[1]), now).await.unwrap();
    repo.apply(ROOM, &add(VIDEOS[2]), now).await.unwrap();
    let play_now = RoomOp::Add {
        video_id: VIDEOS[1].to_owned(),
        added_by: "Ada".into(),
        max_queue: 3,
        play: true,
    };
    repo.apply(ROOM, &play_now, now).await.unwrap();
    let (current_now, ids, playing) = queue(&repo).await;
    assert!(playing && current_now != current);
    assert_eq!(ids, [VIDEOS[1], VIDEOS[1], VIDEOS[2]]);
    let s = repo.snapshot(ROOM).await.unwrap().unwrap();
    assert_eq!(
        Some(s.queue[0].entry_id),
        current_now,
        "the new entry plays first"
    );
    assert!(s.queue[0].entry_id > s.queue[1].entry_id);

    assert_eq!(
        repo.apply("rrrrrrrrrrrrrrrrrrrrrX", &play, now)
            .await
            .unwrap(),
        OpOutcome::NotFound
    );
    assert!(!repo.delete(user(&db.pool).await.id, ROOM).await.unwrap());
    assert!(repo.delete(owner.id, ROOM).await.unwrap());
    assert!(repo.get(ROOM).await.unwrap().is_none());
}

fn add_many(video_ids: &[&str], max_queue: u32) -> RoomOp {
    RoomOp::AddMany {
        video_ids: video_ids.iter().map(|&id| id.to_owned()).collect(),
        added_by: "Ada".into(),
        max_queue,
    }
}

async fn entries(repo: &PgRoomRepository) -> (i64, Vec<i64>) {
    let s = repo.snapshot(ROOM).await.unwrap().unwrap();
    (
        s.room.revision,
        s.queue.iter().map(|e| e.entry_id).collect(),
    )
}

#[tokio::test]
#[ignore = "requires Docker or OXISTREAM_TEST_DATABASE_URL"]
async fn queue_fills_in_batches_and_reorders() {
    let db = isolated_test_pool().await;
    let repo = PgRoomRepository::new(db.pool.clone());
    let owner = user(&db.pool).await;
    cache_videos(&db.pool).await;
    let now = OffsetDateTime::now_utc();
    repo.create(owner.id, ROOM, None, 1).await.unwrap();

    // Into an empty room, the first becomes current, paused; the rest of the batch is cut off.
    assert_eq!(
        repo.apply(ROOM, &add_many(&VIDEOS, 5), now).await.unwrap(),
        OpOutcome::Added(5)
    );
    let (current, ids, playing) = queue(&repo).await;
    assert_eq!(ids, &VIDEOS[..5]);
    assert!(current.is_some() && !playing);
    assert!(matches!(
        repo.apply(ROOM, &add_many(&VIDEOS[5..], 5), now).await,
        Err(Error::LimitReached)
    ));

    // Upcoming entries move by index among themselves; the current one stays first.
    let (revision, before) = entries(&repo).await;
    let last = before[4];
    let to_front = RoomOp::Move {
        entry: last,
        to: 0,
        revision,
    };
    assert_eq!(
        repo.apply(ROOM, &to_front, now).await.unwrap(),
        OpOutcome::Applied
    );
    let (_, after) = entries(&repo).await;
    assert_eq!(after, [before[0], last, before[1], before[2], before[3]]);
    assert_eq!(
        repo.apply(ROOM, &to_front, now).await.unwrap(),
        OpOutcome::Stale,
        "the room moved on since"
    );
    assert_eq!(entries(&repo).await.1, after);

    let (revision, _) = entries(&repo).await;
    let current_entry = RoomOp::Move {
        entry: before[0],
        to: 2,
        revision,
    };
    assert_eq!(
        repo.apply(ROOM, &current_entry, now).await.unwrap(),
        OpOutcome::Unchanged
    );
    let to_end = RoomOp::Move {
        entry: last,
        to: 99,
        revision,
    };
    repo.apply(ROOM, &to_end, now).await.unwrap();
    assert_eq!(
        entries(&repo).await.1,
        [before[0], before[1], before[2], before[3], last]
    );

    // The queue advances in its new order, and new entries go to the end.
    let (revision, _) = entries(&repo).await;
    let move_up = RoomOp::Move {
        entry: before[3],
        to: 0,
        revision,
    };
    repo.apply(ROOM, &move_up, now).await.unwrap();
    repo.apply(ROOM, &RoomOp::Advance { from: before[0] }, now)
        .await
        .unwrap();
    let (current, ..) = queue(&repo).await;
    assert_eq!(current, Some(before[3]));
    repo.apply(ROOM, &RoomOp::Remove { entry: last }, now)
        .await
        .unwrap();
    let append = RoomOp::Add {
        video_id: VIDEOS[5].to_owned(),
        added_by: "Ada".into(),
        max_queue: 5,
        play: false,
    };
    repo.apply(ROOM, &append, now).await.unwrap();
    let (_, ids, _) = queue(&repo).await;
    assert_eq!(ids, [VIDEOS[3], VIDEOS[1], VIDEOS[2], VIDEOS[5]]);
}

#[tokio::test]
#[ignore = "requires Docker or OXISTREAM_TEST_DATABASE_URL"]
async fn shuffles_change_only_the_upcoming_order() {
    let db = isolated_test_pool().await;
    let repo = PgRoomRepository::new(db.pool.clone());
    let owner = user(&db.pool).await;
    cache_videos(&db.pool).await;
    let now = OffsetDateTime::now_utc();
    repo.create(owner.id, ROOM, None, 1).await.unwrap();
    repo.apply(ROOM, &add_many(&VIDEOS[..4], 5), now)
        .await
        .unwrap();

    // Shuffling always changes the order of the upcoming entries, never the current one.
    let (_, before) = entries(&repo).await;
    for _ in 0..5 {
        let (_, previous) = entries(&repo).await;
        assert_eq!(
            repo.apply(ROOM, &RoomOp::Shuffle, now).await.unwrap(),
            OpOutcome::Applied
        );
        let (_, shuffled) = entries(&repo).await;
        assert_eq!(shuffled[0], before[0]);
        assert_ne!(shuffled, previous);
        let mut sorted = shuffled.clone();
        sorted.sort_unstable();
        let mut expected = before.clone();
        expected.sort_unstable();
        assert_eq!(sorted, expected);
    }
    for entry in &before[2..] {
        repo.apply(ROOM, &RoomOp::Remove { entry: *entry }, now)
            .await
            .unwrap();
    }
    assert_eq!(
        repo.apply(ROOM, &RoomOp::Shuffle, now).await.unwrap(),
        OpOutcome::Unchanged,
        "one upcoming entry has no other order"
    );
}

#[tokio::test]
#[ignore = "requires Docker or OXISTREAM_TEST_DATABASE_URL"]
async fn participants_are_capped_kicked_and_cleaned_up() {
    let db = isolated_test_pool().await;
    let repo = PgRoomRepository::new(db.pool.clone());
    let owner = user(&db.pool).await;
    repo.create(owner.id, ROOM, None, 10).await.unwrap();

    let a = guest("g:a");
    let b = guest("g:b");
    assert_eq!(repo.join(ROOM, &a, 1).await.unwrap(), JoinOutcome::Joined);
    assert_eq!(repo.join(ROOM, &b, 1).await.unwrap(), JoinOutcome::Full);
    assert_eq!(repo.join(ROOM, &b, 2).await.unwrap(), JoinOutcome::Joined);
    assert_eq!(
        repo.join("rrrrrrrrrrrrrrrrrrrrrX", &b, 2).await.unwrap(),
        JoinOutcome::NotFound
    );

    let kick = RoomOp::Kick { participant: a.id };
    repo.apply(ROOM, &kick, OffsetDateTime::now_utc())
        .await
        .unwrap();
    let s = repo.snapshot(ROOM).await.unwrap().unwrap();
    assert_eq!(
        s.participants.iter().map(|p| p.id).collect::<Vec<_>>(),
        [b.id]
    );
    assert_eq!(s.room.banned, ["g:a"]);
    assert_eq!(
        repo.join(ROOM, &guest("g:a"), 10).await.unwrap(),
        JoinOutcome::Banned
    );

    repo.leave(b.id).await.unwrap();
    let s = repo.snapshot(ROOM).await.unwrap().unwrap();
    assert_eq!(s.participants, []);

    // Stale participants go first, then rooms nobody is in.
    repo.join(ROOM, &guest("g:c"), 10).await.unwrap();
    assert_eq!(
        repo.cleanup(Duration::from_hours(1), Duration::ZERO)
            .await
            .unwrap(),
        0
    );
    assert_eq!(
        repo.cleanup(Duration::ZERO, Duration::ZERO).await.unwrap(),
        1
    );
    assert!(repo.get(ROOM).await.unwrap().is_none());
}

type Socket = WebSocketStream<MaybeTlsStream<TcpStream>>;

async fn connect(base: &reqwest::Url, room: &str, token: Option<&str>) -> Socket {
    let url = format!(
        "ws://{}/api/v0/rooms/{room}/ws",
        base.host_str()
            .zip(base.port())
            .map_or_else(String::new, |(h, p)| format!("{h}:{p}"))
    );
    let mut request = url.into_client_request().unwrap();
    let origin = base.as_str().trim_end_matches('/').to_owned();
    request
        .headers_mut()
        .insert("origin", origin.parse().unwrap());
    if let Some(token) = token {
        request.headers_mut().insert(
            "cookie",
            format!("oxistream_session={token}").parse().unwrap(),
        );
    }
    tokio_tungstenite::connect_async(request).await.unwrap().0
}

async fn send(socket: &mut Socket, message: Value) {
    socket
        .send(Message::Text(message.to_string().into()))
        .await
        .unwrap();
}

/// The next message of `kind`, skipping others.
async fn expect(socket: &mut Socket, kind: &str) -> Value {
    tokio::time::timeout(Duration::from_secs(10), async {
        loop {
            match socket.next().await {
                Some(Ok(Message::Text(text))) => {
                    let value: Value = serde_json::from_str(&text).unwrap();
                    if value["type"] == kind {
                        return value;
                    }
                }
                Some(Ok(_)) => {}
                other => panic!("connection ended waiting for {kind}: {other:?}"),
            }
        }
    })
    .await
    .unwrap_or_else(|_| panic!("timed out waiting for {kind}"))
}

/// The next state that satisfies `check`.
async fn state_where(socket: &mut Socket, check: impl Fn(&Value) -> bool) -> Value {
    loop {
        let state = expect(socket, "state").await;
        if check(&state["room"]) {
            return state["room"].clone();
        }
    }
}

/// An instance of the server with its own room registry, as another replica would be.
async fn replica(pool: &PgPool) -> (Router, reqwest::Url) {
    replica_with(pool, None).await
}

async fn replica_with(
    pool: &PgPool,
    related: Option<Arc<dyn RelatedSource>>,
) -> (Router, reqwest::Url) {
    let mut state = state_with_pool(pool.clone());
    state.rooms = Rooms::new(Arc::new(PgRoomRepository::new(pool.clone())), related);
    state.rooms.spawn(pool.clone());
    let app = oxistream_server::app(state);
    (app.clone(), common::spawn_upstream(app).await)
}

async fn rest(
    app: &Router,
    method: Method,
    uri: &str,
    token: &str,
    body: Option<Value>,
) -> (StatusCode, Value) {
    let mut req = Request::builder()
        .method(method)
        .uri(uri)
        .header(header::HOST, "tube.test")
        .header(header::ORIGIN, "http://tube.test")
        .header(header::COOKIE, format!("oxistream_session={token}"));
    if body.is_some() {
        req = req.header(header::CONTENT_TYPE, "application/json");
    }
    let body = body.map_or_else(Body::empty, |b| Body::from(b.to_string()));
    let response = app.clone().oneshot(req.body(body).unwrap()).await.unwrap();
    let status = response.status();
    let bytes = response.into_body().collect().await.unwrap().to_bytes();
    (
        status,
        serde_json::from_slice(&bytes).unwrap_or(Value::Null),
    )
}

#[tokio::test]
#[ignore = "requires Docker or OXISTREAM_TEST_DATABASE_URL"]
async fn replicas_stay_in_sync_through_the_database() {
    let db = isolated_test_pool().await;
    cache_videos(&db.pool).await;
    let owner = user(&db.pool).await;
    let token = session(&db.pool, &owner).await;
    let (app, a) = replica(&db.pool).await;
    let (_, b) = replica(&db.pool).await;

    let (status, created) = rest(
        &app,
        Method::POST,
        "/api/v0/rooms",
        &token,
        Some(json!({ "video_id": VIDEOS[0] })),
    )
    .await;
    assert_eq!(status, StatusCode::CREATED);
    let room = created["id"].as_str().unwrap().to_owned();

    let mut host = connect(&a, &room, Some(&token)).await;
    send(&mut host, json!({ "type": "join" })).await;
    let welcome = expect(&mut host, "welcome").await;
    assert_eq!(welcome["is_owner"], true);

    let mut guest = connect(&b, &room, None).await;
    send(
        &mut guest,
        json!({ "type": "join", "name": "Grace", "guest_key": "guest-key-0123456789" }),
    )
    .await;
    let welcome = expect(&mut guest, "welcome").await;
    assert_eq!(welcome["is_owner"], false);
    let guest_id = welcome["participant_id"].as_str().unwrap().to_owned();
    let state = state_where(&mut guest, |r| {
        r["participants"].as_array().unwrap().len() == 2
    })
    .await;
    assert_eq!(state["current"]["video"]["id"], VIDEOS[0]);
    assert_eq!(state["participants"][1]["name"], "Grace");
    assert_eq!(state["participants"][1]["is_guest"], true);

    // The owner on replica A sees the guest join on replica B, and the other way round.
    state_where(&mut host, |r| {
        r["participants"].as_array().unwrap().len() == 2
    })
    .await;
    send(&mut host, json!({ "type": "play", "position": 12.5 })).await;
    // States queued before an action may still arrive first, so wait for its exact result.
    state_where(&mut guest, |r| {
        r["playing"] == true && r["position"] == 12.5
    })
    .await;

    send(&mut guest, json!({ "type": "pause", "position": 20.0 })).await;
    state_where(&mut host, |r| {
        r["playing"] == false && r["position"] == 20.0
    })
    .await;

    // A seek dated back by the sender's latency keeps that time.
    let at = i64::try_from(time::OffsetDateTime::now_utc().unix_timestamp_nanos() / 1_000_000)
        .unwrap()
        - 300;
    send(
        &mut host,
        json!({ "type": "seek", "position": 21.0, "at": at }),
    )
    .await;
    let state = state_where(&mut guest, |r| r["position"] == 21.0).await;
    assert_eq!(state["position_at"], at);

    send(&mut guest, json!({ "type": "add", "video_id": VIDEOS[1] })).await;
    let state = state_where(&mut host, |r| r["queue"].as_array().unwrap().len() == 1).await;
    assert_eq!(state["queue"][0]["video"]["id"], VIDEOS[1]);
    assert_eq!(state["queue"][0]["added_by"], "Grace");

    let entry = state["current"]["id"].clone();
    send(&mut host, json!({ "type": "ended", "entry": entry })).await;
    send(&mut guest, json!({ "type": "ended", "entry": entry })).await;
    let state = state_where(&mut guest, |r| r["current"]["video"]["id"] == VIDEOS[1]).await;
    assert_eq!(state["playing"], true);
    assert_eq!(state["queue"], json!([]));

    let play_now = json!({ "type": "add", "video_id": VIDEOS[2], "play": true });
    send(&mut guest, play_now).await;
    let state = state_where(&mut host, |r| r["current"]["video"]["id"] == VIDEOS[2]).await;
    assert!(state["playing"] == true && state["queue"] == json!([]));

    // Only the owner may remove participants.
    send(
        &mut guest,
        json!({ "type": "kick", "participant": guest_id }),
    )
    .await;
    assert_eq!(expect(&mut guest, "error").await["code"], "forbidden");
    send(
        &mut host,
        json!({ "type": "kick", "participant": guest_id }),
    )
    .await;
    expect(&mut guest, "kicked").await;
    state_where(&mut host, |r| {
        r["participants"].as_array().unwrap().len() == 1
    })
    .await;

    let (status, listed) = rest(&app, Method::GET, "/api/v0/rooms", &token, None).await;
    assert_eq!(status, StatusCode::OK);
    assert_eq!(listed[0]["id"], room.as_str());
    assert_eq!(listed[0]["participant_count"], 1);
    let uri = format!("/api/v0/rooms/{room}");
    let (status, _) = rest(&app, Method::DELETE, &uri, &token, None).await;
    assert_eq!(status, StatusCode::NO_CONTENT);
    expect(&mut host, "closed").await;
}

#[tokio::test]
#[ignore = "requires Docker or OXISTREAM_TEST_DATABASE_URL"]
async fn sockets_need_this_origin_and_a_valid_join() {
    let db = isolated_test_pool().await;
    let owner = user(&db.pool).await;
    PgRoomRepository::new(db.pool.clone())
        .create(owner.id, ROOM, None, 10)
        .await
        .unwrap();
    let (_, a) = replica(&db.pool).await;

    let url = format!(
        "ws://{}:{}/api/v0/rooms/{ROOM}/ws",
        a.host_str().unwrap(),
        a.port().unwrap()
    );
    let mut request = url.clone().into_client_request().unwrap();
    request
        .headers_mut()
        .insert("origin", "https://evil.example".parse().unwrap());
    let err = tokio_tungstenite::connect_async(request).await.unwrap_err();
    assert!(err.to_string().contains("403"), "{err}");
    let err = tokio_tungstenite::connect_async(url).await.unwrap_err();
    assert!(err.to_string().contains("403"), "{err}");

    let mut socket = connect(&a, ROOM, None).await;
    send(&mut socket, json!({ "type": "join", "name": "" })).await;
    assert_eq!(expect(&mut socket, "error").await["code"], "invalid");

    let mut socket = connect(&a, "rrrrrrrrrrrrrrrrrrrrrX", None).await;
    send(
        &mut socket,
        json!({ "type": "join", "name": "Grace", "guest_key": "guest-key-0123456789" }),
    )
    .await;
    assert_eq!(expect(&mut socket, "error").await["code"], "not_found");
}

#[tokio::test]
#[ignore = "requires Docker or OXISTREAM_TEST_DATABASE_URL"]
async fn sockets_queue_batches_reorder_and_shuffle() {
    let db = isolated_test_pool().await;
    cache_videos(&db.pool).await;
    let owner = user(&db.pool).await;
    let token = session(&db.pool, &owner).await;
    PgRoomRepository::new(db.pool.clone())
        .create(owner.id, ROOM, Some((VIDEOS[0], "Ada")), 10)
        .await
        .unwrap();
    let (_, a) = replica(&db.pool).await;
    let (_, b) = replica(&db.pool).await;

    let mut host = connect(&a, ROOM, Some(&token)).await;
    send(&mut host, json!({ "type": "join" })).await;
    expect(&mut host, "welcome").await;
    let mut guest = connect(&b, ROOM, None).await;
    send(
        &mut guest,
        json!({ "type": "join", "name": "Grace", "guest_key": "guest-key-0123456789" }),
    )
    .await;
    expect(&mut guest, "welcome").await;

    send(
        &mut guest,
        json!({ "type": "add_many", "video_ids": &VIDEOS[1..4] }),
    )
    .await;
    let added = expect(&mut guest, "added").await;
    assert_eq!(
        (added["added"].clone(), added["skipped"].clone()),
        (json!(3), json!(0))
    );
    let ids = |room: &Value| -> Vec<String> {
        room["queue"]
            .as_array()
            .unwrap()
            .iter()
            .map(|e| e["video"]["id"].as_str().unwrap().to_owned())
            .collect()
    };
    let state = state_where(&mut host, |r| r["queue"].as_array().unwrap().len() == 3).await;
    assert_eq!(ids(&state), &VIDEOS[1..4]);
    assert_eq!(state["queue"][0]["added_by"], "Grace");

    for invalid in [json!([]), json!(["short"])] {
        send(
            &mut guest,
            json!({ "type": "add_many", "video_ids": invalid }),
        )
        .await;
        assert_eq!(expect(&mut guest, "error").await["code"], "invalid");
    }

    // A move based on the latest state applies on the other replica too; a stale one does not.
    let last = state["queue"][2]["id"].clone();
    let revision = state["revision"].clone();
    send(
        &mut host,
        json!({ "type": "move", "entry": last, "to": 0, "revision": revision }),
    )
    .await;
    let moved = [VIDEOS[3], VIDEOS[1], VIDEOS[2]];
    let state = state_where(&mut guest, |r| ids(r) == moved).await;
    send(
        &mut guest,
        json!({ "type": "move", "entry": last, "to": 2, "revision": revision }),
    )
    .await;
    assert_eq!(expect(&mut guest, "error").await["code"], "stale");

    send(&mut guest, json!({ "type": "shuffle" })).await;
    let shuffled = state_where(&mut host, |r| {
        r["revision"].as_i64() > state["revision"].as_i64()
    })
    .await;
    assert_ne!(ids(&shuffled), moved);
    let mut sorted = ids(&shuffled);
    sorted.sort();
    assert_eq!(sorted, &VIDEOS[1..4]);
    assert_eq!(shuffled["current"]["video"]["id"], VIDEOS[0]);
}

fn related_video(id: &str) -> VideoUpsert {
    VideoUpsert {
        id: id.to_owned(),
        title: format!("Related {id}"),
        channel_id: "UCbbbbbbbbbbbbbbbbbbbbbb".into(),
        channel_name: "Beta".into(),
        duration_seconds: Some(95),
        published_at: None,
        thumbnail_url: None,
    }
}

const RELATED: [&str; 2] = ["vvvvvvvvvvM", "vvvvvvvvvvQ"];

fn pick(source: i64, ids: &[&str]) -> RoomOp {
    RoomOp::Pick {
        source,
        candidates: ids.iter().map(|id| related_video(id)).collect(),
    }
}

#[tokio::test]
#[ignore = "requires Docker or OXISTREAM_TEST_DATABASE_URL"]
async fn autoplay_appends_related_videos_without_repeats() {
    let db = isolated_test_pool().await;
    let repo = PgRoomRepository::new(db.pool.clone());
    let owner = user(&db.pool).await;
    cache_videos(&db.pool).await;
    let now = OffsetDateTime::now_utc();
    let room = repo
        .create(owner.id, ROOM, Some((VIDEOS[0], "Ada")), 1)
        .await
        .unwrap();
    let first = room.current_entry_id.unwrap();
    repo.apply(ROOM, &add(VIDEOS[1]), now).await.unwrap();

    // Off, or with entries still to come, there is nothing to pick.
    let s = repo.snapshot(ROOM).await.unwrap().unwrap();
    assert_eq!(s.needs_pick(), None);
    assert_eq!(
        repo.apply(ROOM, &pick(first, &RELATED), now).await.unwrap(),
        OpOutcome::Unchanged
    );
    let on = RoomOp::SetAutoplay { enabled: true };
    assert_eq!(
        repo.apply(ROOM, &on, now).await.unwrap(),
        OpOutcome::Applied
    );
    assert_eq!(
        repo.apply(ROOM, &on, now).await.unwrap(),
        OpOutcome::Unchanged
    );
    let s = repo.snapshot(ROOM).await.unwrap().unwrap();
    assert!(s.room.autoplay);
    assert_eq!(s.needs_pick(), None);
    assert_eq!(
        repo.apply(ROOM, &pick(first, &RELATED), now).await.unwrap(),
        OpOutcome::Unchanged
    );

    // Once the last entry plays, the first related video the room has not queued is picked.
    repo.apply(ROOM, &RoomOp::Advance { from: first }, now)
        .await
        .unwrap();
    let s = repo.snapshot(ROOM).await.unwrap().unwrap();
    let second = s.room.current_entry_id.unwrap();
    assert_eq!(s.needs_pick(), Some((second, VIDEOS[1])));
    let picked = pick(second, &[VIDEOS[0], VIDEOS[1], RELATED[0], RELATED[1]]);
    assert_eq!(
        repo.apply(ROOM, &picked, now).await.unwrap(),
        OpOutcome::Applied
    );
    assert_eq!(
        repo.apply(ROOM, &pick(second, &[RELATED[1]]), now)
            .await
            .unwrap(),
        OpOutcome::Unchanged,
        "the first pick stays"
    );
    let s = repo.snapshot(ROOM).await.unwrap().unwrap();
    assert_eq!(s.needs_pick(), None);
    assert_eq!(s.up_next.as_ref().map(|v| v.id.as_str()), Some(RELATED[0]));

    // Turned off, there is no preview; turned on again, the pick is still there.
    let off = RoomOp::SetAutoplay { enabled: false };
    repo.apply(ROOM, &off, now).await.unwrap();
    assert_eq!(repo.snapshot(ROOM).await.unwrap().unwrap().up_next, None);
    repo.apply(ROOM, &on, now).await.unwrap();

    repo.apply(ROOM, &RoomOp::Advance { from: second }, now)
        .await
        .unwrap();
    let s = repo.snapshot(ROOM).await.unwrap().unwrap();
    assert!(s.room.playing);
    assert_eq!(s.queue.len(), 1);
    assert_eq!(s.queue[0].video.id, RELATED[0]);
    assert!(s.queue[0].autoplay);
    assert_eq!(s.needs_pick(), Some((s.queue[0].entry_id, RELATED[0])));
}

#[tokio::test]
#[ignore = "requires Docker or OXISTREAM_TEST_DATABASE_URL"]
async fn autoplay_never_repeats_a_queued_video() {
    let db = isolated_test_pool().await;
    let repo = PgRoomRepository::new(db.pool.clone());
    let owner = user(&db.pool).await;
    cache_videos(&db.pool).await;
    let now = OffsetDateTime::now_utc();
    let room = repo
        .create(owner.id, ROOM, Some((VIDEOS[0], "Ada")), 1)
        .await
        .unwrap();
    let third = room.current_entry_id.unwrap();
    repo.apply(ROOM, &RoomOp::SetAutoplay { enabled: true }, now)
        .await
        .unwrap();

    // Queueing the pick by hand drops it, so it is not played twice.
    repo.apply(ROOM, &pick(third, &[RELATED[1]]), now)
        .await
        .unwrap();
    let manual = RoomOp::Add {
        video_id: RELATED[1].to_owned(),
        added_by: "Ada".into(),
        max_queue: 3,
        play: false,
    };
    repo.apply(ROOM, &manual, now).await.unwrap();
    let s = repo.snapshot(ROOM).await.unwrap().unwrap();
    assert_eq!(s.room.autoplay_video_id, None);
    repo.apply(
        ROOM,
        &RoomOp::Remove {
            entry: s.queue[1].entry_id,
        },
        now,
    )
    .await
    .unwrap();

    // With every candidate queued before, nothing is picked and the queue runs out.
    let s = repo.snapshot(ROOM).await.unwrap().unwrap();
    assert_eq!(s.needs_pick(), Some((third, VIDEOS[0])));
    let seen = pick(third, &[VIDEOS[0], RELATED[1]]);
    assert_eq!(
        repo.apply(ROOM, &seen, now).await.unwrap(),
        OpOutcome::Applied
    );
    let s = repo.snapshot(ROOM).await.unwrap().unwrap();
    assert_eq!((s.needs_pick(), &s.up_next), (None, &None));
    repo.apply(ROOM, &RoomOp::Advance { from: third }, now)
        .await
        .unwrap();
    let (current, ids, _) = queue(&repo).await;
    assert!(current.is_none() && ids.is_empty());
}

struct FakeRelated;

#[async_trait::async_trait]
impl RelatedSource for FakeRelated {
    async fn related(&self, _video_id: &str) -> youtube::Result<Vec<VideoUpsert>> {
        Ok([VIDEOS[0], RELATED[0], RELATED[1]]
            .iter()
            .map(|id| related_video(id))
            .collect())
    }
}

#[tokio::test]
#[ignore = "requires Docker or OXISTREAM_TEST_DATABASE_URL"]
async fn sockets_toggle_autoplay_and_play_the_preview() {
    let db = isolated_test_pool().await;
    cache_videos(&db.pool).await;
    let owner = user(&db.pool).await;
    let token = session(&db.pool, &owner).await;
    PgRoomRepository::new(db.pool.clone())
        .create(owner.id, ROOM, Some((VIDEOS[0], "Ada")), 10)
        .await
        .unwrap();
    let (_, a) = replica_with(&db.pool, Some(Arc::new(FakeRelated))).await;
    let (_, b) = replica_with(&db.pool, Some(Arc::new(FakeRelated))).await;

    let mut host = connect(&a, ROOM, Some(&token)).await;
    send(&mut host, json!({ "type": "join" })).await;
    expect(&mut host, "welcome").await;
    let mut guest = connect(&b, ROOM, None).await;
    send(
        &mut guest,
        json!({ "type": "join", "name": "Grace", "guest_key": "guest-key-0123456789" }),
    )
    .await;
    expect(&mut guest, "welcome").await;
    let state = state_where(&mut guest, |r| {
        r["participants"].as_array().unwrap().len() == 2
    })
    .await;
    assert_eq!(
        (state["autoplay"].clone(), state["up_next"].clone()),
        (json!(false), Value::Null)
    );

    // Guests may turn it on too; both replicas see the same preview.
    send(&mut guest, json!({ "type": "autoplay", "enabled": true })).await;
    let state = state_where(&mut host, |r| !r["up_next"].is_null()).await;
    assert_eq!(state["autoplay"], true);
    assert_eq!(state["up_next"]["id"], RELATED[0]);
    state_where(&mut guest, |r| r["up_next"]["id"] == RELATED[0]).await;

    let entry = state["current"]["id"].clone();
    send(&mut guest, json!({ "type": "ended", "entry": entry })).await;
    let state = state_where(&mut host, |r| r["current"]["video"]["id"] == RELATED[0]).await;
    assert_eq!(state["current"]["autoplay"], true);
    assert_eq!(state["current"]["added_by"], "");
    assert_eq!(state["playing"], true);
    let state = state_where(&mut guest, |r| r["up_next"]["id"] == RELATED[1]).await;
    assert_eq!(state["queue"], json!([]));

    send(&mut host, json!({ "type": "autoplay", "enabled": false })).await;
    state_where(&mut guest, |r| {
        r["autoplay"] == false && r["up_next"].is_null()
    })
    .await;
}
