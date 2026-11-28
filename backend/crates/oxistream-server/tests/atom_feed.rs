//! Needs `OXISTREAM_TEST_DATABASE_URL` or a Docker daemon; run with `cargo test -- --include-ignored`.

mod common;

use axum::Router;
use axum::body::Body;
use axum::http::{Method, Request, StatusCode, header};
use common::{isolated_test_pool, state_with_auth, unique_suffix};
use http_body_util::BodyExt;
use oxistream_db::{
    ChannelRepository, ChannelUpdate, ChannelVideoUpsert, Image, NewSession, NewUser,
    PgChannelRepository, PgPool, PgSessionRepository, PgSubscriptionRepository, PgUserRepository,
    PgUserSettingsRepository, SessionRepository, SubscriptionRepository, User, UserRepository,
    UserSettingsRepository,
};
use oxistream_server::auth::cookie;
use oxistream_server::config::AuthConfig;
use quick_xml::events::Event;
use serde_json::Value;
use time::OffsetDateTime;
use time::macros::datetime;
use tower::ServiceExt;

const CHANNEL: &str = "UCaaaaaaaaaaaaaaaaaaaaaa";

async fn user(pool: &PgPool) -> User {
    PgUserRepository::new(pool.clone())
        .create(&NewUser {
            username: format!("u_{}", unique_suffix()),
            password_hash: None,
            admin_if_none: true,
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

fn video(id: &str, title: &str, published_at: OffsetDateTime) -> ChannelVideoUpsert {
    ChannelVideoUpsert {
        video_id: id.to_owned(),
        title: title.to_owned(),
        duration_seconds: Some(213),
        published_at: Some(published_at),
        published_text: None,
        thumbnails: vec![Image {
            url: format!("https://i.ytimg.com/vi/{id}/hqdefault.jpg"),
            width: 480,
            height: 360,
        }],
        view_count: Some(42),
        is_live: false,
        is_upcoming: false,
        is_short: false,
    }
}

struct Response {
    status: StatusCode,
    headers: axum::http::HeaderMap,
    body: String,
}

impl Response {
    fn json(&self) -> Value {
        serde_json::from_str(&self.body).unwrap_or(Value::Null)
    }
}

async fn send(app: &Router, method: Method, uri: &str, session: Option<&str>) -> Response {
    let mut req = Request::builder()
        .method(method)
        .uri(uri)
        .header(header::HOST, "tube.test");
    if let Some(token) = session {
        req = req
            .header(header::ORIGIN, "http://tube.test")
            .header(header::COOKIE, format!("oxistream_session={token}"));
    }
    let response = app
        .clone()
        .oneshot(req.body(Body::empty()).unwrap())
        .await
        .unwrap();
    let status = response.status();
    let headers = response.headers().clone();
    let bytes = response.into_body().collect().await.unwrap().to_bytes();
    Response {
        status,
        headers,
        body: String::from_utf8(bytes.to_vec()).unwrap(),
    }
}

/// Parses the feed and returns the text of every element named `name`.
fn texts(xml: &str, name: &str) -> Vec<String> {
    let mut reader = quick_xml::Reader::from_str(xml);
    reader.config_mut().check_end_names = true;
    let (mut out, mut current, mut inside) = (Vec::new(), String::new(), false);
    loop {
        match reader.read_event().expect("well-formed XML") {
            Event::Start(e) if e.name().as_ref() == name => {
                inside = true;
                current.clear();
            }
            Event::Text(t) if inside => current.push_str(&t.xml10_content()),
            Event::GeneralRef(r) if inside => {
                let entity = format!("&{};", r.xml10_content());
                current.push_str(&quick_xml::escape::unescape(&entity).unwrap());
            }
            Event::End(e) if e.name().as_ref() == name => {
                inside = false;
                out.push(current.clone());
            }
            Event::Eof => break,
            _ => {}
        }
    }
    out
}

#[tokio::test]
#[ignore = "requires Docker or OXISTREAM_TEST_DATABASE_URL"]
async fn feed_tokens_are_replaced_and_revoked() {
    let db = isolated_test_pool().await;
    let repo = PgUserSettingsRepository::new(db.pool.clone());
    let users = PgUserRepository::new(db.pool.clone());
    let alice = user(&db.pool).await;
    let bob = user(&db.pool).await;

    assert_eq!(repo.feed_token_created_at(alice.id).await.unwrap(), None);
    let (_, first) = cookie::new_token();
    let (_, second) = cookie::new_token();
    let created = repo.replace_feed_token(alice.id, &first).await.unwrap();
    assert_eq!(
        repo.feed_token_created_at(alice.id).await.unwrap(),
        Some(created)
    );
    assert_eq!(repo.feed_token_user(&first).await.unwrap(), Some(alice.id));

    repo.replace_feed_token(alice.id, &second).await.unwrap();
    assert_eq!(repo.feed_token_user(&first).await.unwrap(), None);
    assert_eq!(repo.feed_token_user(&second).await.unwrap(), Some(alice.id));
    assert_eq!(repo.feed_token_created_at(bob.id).await.unwrap(), None);

    users.set_disabled(alice.id, true).await.unwrap();
    assert_eq!(repo.feed_token_user(&second).await.unwrap(), None);
    users.set_disabled(alice.id, false).await.unwrap();

    assert!(repo.delete_feed_token(alice.id).await.unwrap());
    assert!(!repo.delete_feed_token(alice.id).await.unwrap());
    assert_eq!(repo.feed_token_user(&second).await.unwrap(), None);
}

/// Alice, subscribed to one channel with two videos, on an instance that requires login.
async fn setup() -> (common::TestDb, Router, User, String) {
    let db = isolated_test_pool().await;
    let alice = user(&db.pool).await;
    let cookie = session(&db.pool, &alice).await;
    PgSubscriptionRepository::new(db.pool.clone())
        .subscribe(alice.id, CHANNEL)
        .await
        .unwrap();
    PgChannelRepository::new(db.pool.clone())
        .store_fetch(
            &ChannelUpdate {
                id: CHANNEL.into(),
                name: "Tom & Jerry".into(),
                handle: None,
                avatar: vec![],
                subscriber_count: None,
                verified: false,
            },
            &[
                video("aaaaaaaaaa1", "Older", datetime!(2026-01-01 00:00 UTC)),
                video(
                    "aaaaaaaaaa2",
                    "<Newer> & \"quoted\"",
                    datetime!(2026-02-01 00:00 UTC),
                ),
            ],
        )
        .await
        .unwrap();
    // Login is required for everything else; the token alone opens the feed.
    let app = oxistream_server::app(state_with_auth(
        db.pool.clone(),
        AuthConfig {
            require_login: true,
            ..AuthConfig::default()
        },
    ));

    (db, app, alice, cookie)
}

const TOKEN_PATH: &str = "/api/v0/settings/feed-token";

async fn create(app: &Router, cookie: &str) -> String {
    let created = send(app, Method::POST, TOKEN_PATH, Some(cookie)).await;
    assert_eq!(created.status, StatusCode::CREATED);
    let url = created.json()["url"].as_str().unwrap().to_owned();
    url.strip_prefix("http://tube.test")
        .expect("absolute URL of this instance, from the request's origin")
        .to_owned()
}

#[tokio::test]
#[ignore = "requires Docker or OXISTREAM_TEST_DATABASE_URL"]
async fn atom_feed_lists_subscriptions_by_token() {
    let (_db, app, _alice, cookie) = setup().await;
    let status = send(&app, Method::GET, TOKEN_PATH, None).await;
    assert_eq!(status.status, StatusCode::UNAUTHORIZED);
    let status = send(&app, Method::GET, TOKEN_PATH, Some(&cookie)).await;
    assert_eq!(status.status, StatusCode::OK);
    assert_eq!(status.json()["created_at"], Value::Null);

    let path = create(&app, &cookie).await;
    assert!(path.starts_with("/api/v0/feed/atom/"));
    let status = send(&app, Method::GET, TOKEN_PATH, Some(&cookie)).await;
    assert!(status.json()["created_at"].is_string());

    let feed = send(&app, Method::GET, &path, None).await;
    assert_eq!(feed.status, StatusCode::OK);
    assert_eq!(
        feed.headers[header::CONTENT_TYPE],
        "application/atom+xml; charset=utf-8"
    );
    assert_eq!(feed.headers[header::CACHE_CONTROL], "private, max-age=300");
    assert_eq!(
        texts(&feed.body, "title")[1..],
        ["<Newer> & \"quoted\"", "Older"]
    );
    assert_eq!(texts(&feed.body, "name"), ["Tom & Jerry", "Tom & Jerry"]);
    assert_eq!(
        texts(&feed.body, "published"),
        ["2026-02-01T00:00:00Z", "2026-01-01T00:00:00Z"]
    );
    assert_eq!(
        texts(&feed.body, "id")[1..],
        ["yt:video:aaaaaaaaaa2", "yt:video:aaaaaaaaaa1"]
    );
    assert!(
        feed.body
            .contains("href=\"https://tube.test/watch?v=aaaaaaaaaa2\"")
    );
    assert!(
        feed.body
            .contains("<media:thumbnail url=\"https://tube.test/proxy/image/")
    );
    assert!(
        feed.body
            .contains(&format!("href=\"https://tube.test{path}\"")),
        "self link"
    );
}

#[tokio::test]
#[ignore = "requires Docker or OXISTREAM_TEST_DATABASE_URL"]
async fn regenerating_disabling_and_deleting_close_the_feed() {
    let (db, app, alice, cookie) = setup().await;
    let old = create(&app, &cookie).await;
    let new = create(&app, &cookie).await;
    assert_ne!(new, old);
    assert_eq!(
        send(&app, Method::GET, &old, None).await.status,
        StatusCode::NOT_FOUND
    );
    assert_eq!(
        send(&app, Method::GET, &new, None).await.status,
        StatusCode::OK
    );

    let users = PgUserRepository::new(db.pool.clone());
    users.set_disabled(alice.id, true).await.unwrap();
    assert_eq!(
        send(&app, Method::GET, &new, None).await.status,
        StatusCode::NOT_FOUND
    );
    users.set_disabled(alice.id, false).await.unwrap();

    let deleted = send(&app, Method::DELETE, TOKEN_PATH, Some(&cookie)).await;
    assert_eq!(deleted.status, StatusCode::NO_CONTENT);
    assert_eq!(
        send(&app, Method::GET, &new, None).await.status,
        StatusCode::NOT_FOUND
    );
    for bad in ["/api/v0/feed/atom/nope", "/api/v0/feed/atom/%3Cx%3E"] {
        assert_eq!(
            send(&app, Method::GET, bad, None).await.status,
            StatusCode::NOT_FOUND
        );
    }
}

#[tokio::test]
#[ignore = "requires Docker or OXISTREAM_TEST_DATABASE_URL"]
async fn feed_links_use_the_public_url() {
    let db = isolated_test_pool().await;
    let alice = user(&db.pool).await;
    let cookie = session(&db.pool, &alice).await;
    let app = oxistream_server::app(state_with_auth(
        db.pool.clone(),
        AuthConfig {
            public_origin: Some("https://tube.example.com".into()),
            ..AuthConfig::default()
        },
    ));
    let created = send(&app, Method::POST, TOKEN_PATH, Some(&cookie)).await;
    let url = created.json()["url"].as_str().unwrap().to_owned();
    let path = url.strip_prefix("https://tube.example.com").unwrap();
    let feed = send(&app, Method::GET, path, None).await;
    assert_eq!(feed.status, StatusCode::OK);
    assert!(
        feed.body
            .contains("href=\"https://tube.example.com/feed/subscriptions\"")
    );
    assert!(!feed.body.contains("<entry>"));
}
