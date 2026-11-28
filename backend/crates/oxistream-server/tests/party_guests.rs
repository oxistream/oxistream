//! Party guests with `OXISTREAM_REQUIRE_LOGIN`. Needs `OXISTREAM_TEST_DATABASE_URL` or a
//! Docker daemon; run with `cargo test -- --include-ignored`.

mod common;

use std::fmt::Write as _;
use std::time::Duration;

use axum::Router;
use axum::body::Body;
use axum::http::{Method, Request, StatusCode, header};
use futures_util::{SinkExt, StreamExt};
use http_body_util::BodyExt;
use oxistream_db::{
    NewParticipant, NewUser, PgPool, PgRoomRepository, RoomOp, RoomRepository, User,
    UserRepository, Uuid,
};
use oxistream_server::auth::party_guest::{PartyGuest, PartyGuests};
use oxistream_server::config::AuthConfig;
use serde_json::{Value, json};
use sha2::{Digest, Sha256};
use time::OffsetDateTime;
use tokio_tungstenite::tungstenite::Message;
use tokio_tungstenite::tungstenite::client::IntoClientRequest;
use tower::ServiceExt;

use common::{TEST_PROXY_SECRET, isolated_test_pool, state_with_auth, unique_suffix};

const ROOM: &str = "rrrrrrrrrrrrrrrrrrrrrG";
const OTHER: &str = "rrrrrrrrrrrrrrrrrrrrrH";
const GUEST_KEY: &str = "guest-key-0123456789abcdef";
/// Needs no YouTube request for an empty query.
const ANONYMOUS: &str = "/api/v0/search/suggestions?q=";
/// A bad signature: 403 once past the login check.
const PROXY: &str = "/proxy/image/aHR0cHM6Ly9pLnl0aW1nLmNvbS92aS94L2hxLmpwZw?exp=1&sig=x";
const ACCOUNT_ONLY: [(&str, &str); 4] = [
    ("GET", "/api/v0/subscriptions"),
    ("GET", "/api/v0/history"),
    ("GET", "/api/v0/rooms"),
    ("POST", "/api/v0/rooms"),
];

struct Reply {
    status: StatusCode,
    cookies: Vec<String>,
}

impl Reply {
    /// The value of the party guest cookie this reply sets.
    fn guest_cookie(&self) -> Option<&str> {
        self.cookies.iter().find_map(|c| {
            c.strip_prefix("oxistream_party=")
                .and_then(|rest| rest.split(';').next())
        })
    }
}

fn require_login() -> AuthConfig {
    AuthConfig {
        require_login: true,
        cookie_secure: false,
        ..AuthConfig::default()
    }
}

/// A fresh replica, so nothing it remembers about guests carries over.
fn replica(pool: &PgPool) -> Router {
    oxistream_server::app(state_with_auth(pool.clone(), require_login()))
}

async fn send(
    app: &Router,
    method: &str,
    uri: &str,
    guest_cookie: Option<&str>,
    body: Option<Value>,
) -> Reply {
    let mut req = Request::builder()
        .method(Method::from_bytes(method.as_bytes()).unwrap())
        .uri(uri)
        .header(header::HOST, "tube.test")
        .header(header::ORIGIN, "http://tube.test");
    if let Some(value) = guest_cookie {
        req = req.header(header::COOKIE, format!("oxistream_party={value}"));
    }
    if body.is_some() {
        req = req.header(header::CONTENT_TYPE, "application/json");
    }
    let body = body.map_or_else(Body::empty, |b| Body::from(b.to_string()));
    let response = app.clone().oneshot(req.body(body).unwrap()).await.unwrap();
    let status = response.status();
    let cookies = response
        .headers()
        .get_all(header::SET_COOKIE)
        .iter()
        .map(|v| v.to_str().unwrap().to_owned())
        .collect();
    let _ = response.into_body().collect().await.unwrap();
    Reply { status, cookies }
}

/// Whether a guest with `cookie` gets past the login check, on a fresh replica.
async fn admitted(pool: &PgPool, cookie: Option<&str>) -> bool {
    let app = replica(pool);
    let api = send(&app, "GET", ANONYMOUS, cookie, None).await;
    let proxy = send(&app, "GET", PROXY, cookie, None).await;
    assert_eq!(
        api.status == StatusCode::OK,
        proxy.status == StatusCode::FORBIDDEN,
        "API {} and proxy {} disagree",
        api.status,
        proxy.status
    );
    if api.status == StatusCode::UNAUTHORIZED && cookie.is_some() {
        assert!(
            api.cookies
                .iter()
                .any(|c| c.starts_with("oxistream_party=;") && c.contains("Max-Age=0")),
            "a rejected guest cookie is cleared: {:?}",
            api.cookies
        );
    }
    api.status == StatusCode::OK
}

fn participant_key(guest_key: &str) -> String {
    let hash = Sha256::digest(guest_key.as_bytes());
    let mut key = String::from("g:");
    for b in hash {
        let _ = write!(key, "{b:02x}");
    }
    key
}

fn cookie_seen(secs_ago: u64) -> String {
    let now = OffsetDateTime::now_utc().unix_timestamp().cast_unsigned();
    PartyGuests::new(TEST_PROXY_SECRET.as_bytes()).encode(&PartyGuest {
        room: ROOM.into(),
        key: participant_key(GUEST_KEY),
        seen: now - secs_ago,
    })
}

async fn owner(pool: &PgPool) -> User {
    oxistream_db::PgUserRepository::new(pool.clone())
        .create(&NewUser {
            username: format!("u_{}", unique_suffix()),
            password_hash: None,
            admin_if_none: true,
        })
        .await
        .unwrap()
}

fn guest() -> NewParticipant {
    let mut bytes = [0u8; 16];
    getrandom::fill(&mut bytes).unwrap();
    NewParticipant {
        id: Uuid::from_bytes(bytes),
        key: participant_key(GUEST_KEY),
        user_id: None,
        display_name: "Grace".into(),
    }
}

#[tokio::test]
#[ignore = "requires Docker or OXISTREAM_TEST_DATABASE_URL"]
#[allow(clippy::too_many_lines)]
async fn guests_join_and_browse_while_in_the_party() {
    let db = isolated_test_pool().await;
    let owner = owner(&db.pool).await;
    PgRoomRepository::new(db.pool.clone())
        .create(owner.id, ROOM, None, 10)
        .await
        .unwrap();
    let app = replica(&db.pool);
    let base = common::spawn_upstream(app.clone()).await;

    assert!(!admitted(&db.pool, None).await, "no session, no party");
    let room = send(&app, "GET", &format!("/api/v0/rooms/{ROOM}"), None, None).await;
    assert_eq!(room.status, StatusCode::OK, "the invite link works");
    let early = send(
        &app,
        "POST",
        &format!("/api/v0/rooms/{ROOM}/guest"),
        None,
        Some(json!({ "guest_key": GUEST_KEY })),
    )
    .await;
    assert_eq!(early.status, StatusCode::NOT_FOUND, "not in the room yet");

    let url = format!(
        "ws://{}:{}/api/v0/rooms/{ROOM}/ws",
        base.host_str().unwrap(),
        base.port().unwrap()
    );
    let mut request = url.into_client_request().unwrap();
    let origin = base.as_str().trim_end_matches('/');
    request
        .headers_mut()
        .insert("origin", origin.parse().unwrap());
    let mut socket = tokio_tungstenite::connect_async(request).await.unwrap().0;
    let join = json!({ "type": "join", "name": "Grace", "guest_key": GUEST_KEY });
    socket
        .send(Message::Text(join.to_string().into()))
        .await
        .unwrap();
    tokio::time::timeout(Duration::from_secs(10), async {
        while let Some(Ok(message)) = socket.next().await {
            if let Message::Text(text) = message
                && serde_json::from_str::<Value>(&text).unwrap()["type"] == "welcome"
            {
                return;
            }
        }
        panic!("no welcome");
    })
    .await
    .unwrap();

    let wrong = send(
        &app,
        "POST",
        &format!("/api/v0/rooms/{ROOM}/guest"),
        None,
        Some(json!({ "guest_key": "another-guest-key-0123456789" })),
    )
    .await;
    assert_eq!(
        wrong.status,
        StatusCode::NOT_FOUND,
        "only the guest's own key"
    );
    let issued = send(
        &app,
        "POST",
        &format!("/api/v0/rooms/{ROOM}/guest"),
        None,
        Some(json!({ "guest_key": GUEST_KEY })),
    )
    .await;
    assert_eq!(issued.status, StatusCode::NO_CONTENT);
    let set = issued.cookies.join("\n");
    assert!(
        set.contains("HttpOnly") && set.contains("SameSite=Lax"),
        "{set}"
    );
    let cookie = issued.guest_cookie().unwrap().to_owned();

    assert!(
        admitted(&db.pool, Some(&cookie)).await,
        "a guest in the party"
    );
    let app = replica(&db.pool);
    for (method, uri) in ACCOUNT_ONLY {
        let reply = send(&app, method, uri, Some(&cookie), Some(json!({}))).await;
        assert_eq!(reply.status, StatusCode::UNAUTHORIZED, "{method} {uri}");
    }

    let mut forged = cookie.clone();
    forged.replace_range(..1, if cookie.starts_with('a') { "b" } else { "a" });
    assert!(
        !admitted(&db.pool, Some(&forged)).await,
        "a tampered cookie"
    );

    let mut no_origin = Request::post(format!("/api/v0/rooms/{ROOM}/guest"))
        .header(header::HOST, "tube.test")
        .header(header::CONTENT_TYPE, "application/json")
        .body(Body::from(json!({ "guest_key": GUEST_KEY }).to_string()))
        .unwrap();
    no_origin
        .headers_mut()
        .insert(header::ORIGIN, "https://evil.example".parse().unwrap());
    let response = app.clone().oneshot(no_origin).await.unwrap();
    assert_eq!(response.status(), StatusCode::FORBIDDEN, "origin check");

    let released = send(
        &app,
        "DELETE",
        &format!("/api/v0/rooms/{ROOM}/guest"),
        Some(&cookie),
        None,
    )
    .await;
    assert_eq!(released.status, StatusCode::NO_CONTENT);
    assert!(
        released
            .cookies
            .iter()
            .any(|c| c.starts_with("oxistream_party=;")),
        "{:?}",
        released.cookies
    );
}

#[tokio::test]
#[ignore = "requires Docker or OXISTREAM_TEST_DATABASE_URL"]
async fn guests_are_locked_out_once_out_of_the_party() {
    let db = isolated_test_pool().await;
    let repo = PgRoomRepository::new(db.pool.clone());
    let owner = owner(&db.pool).await;
    repo.create(owner.id, ROOM, None, 10).await.unwrap();
    let fresh = cookie_seen(0);
    let stale = cookie_seen(180);

    let a = guest();
    repo.join(ROOM, &a, 10).await.unwrap();
    assert!(
        admitted(&db.pool, Some(&stale)).await,
        "present in the room"
    );

    // Left, reloading or reconnecting: a short grace, then locked.
    repo.leave(a.id).await.unwrap();
    assert!(admitted(&db.pool, Some(&fresh)).await, "just left");
    assert!(!admitted(&db.pool, Some(&stale)).await, "left a while ago");

    // Dropped after missed heartbeats.
    let b = guest();
    repo.join(ROOM, &b, 10).await.unwrap();
    repo.cleanup(Duration::ZERO, Duration::from_hours(24))
        .await
        .unwrap();
    assert!(!admitted(&db.pool, Some(&stale)).await, "dropped");

    // Kicked: at once, whenever the guest was last seen.
    let c = guest();
    repo.join(ROOM, &c, 10).await.unwrap();
    repo.apply(
        ROOM,
        &RoomOp::Kick { participant: c.id },
        OffsetDateTime::now_utc(),
    )
    .await
    .unwrap();
    assert!(!admitted(&db.pool, Some(&fresh)).await, "kicked");

    // Ended by the owner or expired.
    repo.create(owner.id, OTHER, None, 10).await.unwrap();
    let mut d = guest();
    d.key = participant_key("other-guest-key-0123456789");
    repo.join(OTHER, &d, 10).await.unwrap();
    let other = PartyGuests::new(TEST_PROXY_SECRET.as_bytes()).encode(&PartyGuest {
        room: OTHER.into(),
        key: d.key.clone(),
        seen: OffsetDateTime::now_utc().unix_timestamp().cast_unsigned(),
    });
    assert!(admitted(&db.pool, Some(&other)).await);
    assert!(repo.delete(owner.id, OTHER).await.unwrap());
    assert!(!admitted(&db.pool, Some(&other)).await, "party ended");
}

#[tokio::test]
#[ignore = "requires Docker or OXISTREAM_TEST_DATABASE_URL"]
async fn guest_cookies_are_ignored_without_require_login() {
    let db = isolated_test_pool().await;
    let app = oxistream_server::app(state_with_auth(db.pool.clone(), AuthConfig::default()));
    let reply = send(&app, "GET", ANONYMOUS, Some("garbage"), None).await;
    assert_eq!(reply.status, StatusCode::OK);
    assert!(reply.cookies.is_empty(), "{:?}", reply.cookies);
}
