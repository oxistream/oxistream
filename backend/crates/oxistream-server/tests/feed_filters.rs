//! Needs `OXISTREAM_TEST_DATABASE_URL` or a Docker daemon; run with `cargo test -- --include-ignored`.

mod common;

use axum::Router;
use axum::body::Body;
use axum::http::{Method, Request, StatusCode, header};
use common::{app_with_pool, isolated_test_pool, unique_suffix};
use http_body_util::BodyExt;
use oxistream_db::{
    ChannelRepository, ChannelUpdate, ChannelVideoUpsert, FeedFilter, FeedKind,
    HiddenVideoRepository, HistoryRepository, NewSession, NewUser, PgChannelRepository,
    PgHiddenVideoRepository, PgHistoryRepository, PgPool, PgSessionRepository,
    PgSubscriptionRepository, PgUserRepository, PgUserSettingsRepository, ProgressUpdate,
    SessionRepository, SubscriptionRepository, User, UserRepository, UserSettingsRepository,
};
use oxistream_server::auth::cookie;
use serde_json::{Value, json};
use time::OffsetDateTime;
use time::macros::datetime;
use tower::ServiceExt;

const CHANNEL_A: &str = "UCaaaaaaaaaaaaaaaaaaaaaa";
const CHANNEL_B: &str = "UCbbbbbbbbbbbbbbbbbbbbbb";

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

fn channel(id: &str, name: &str) -> ChannelUpdate {
    ChannelUpdate {
        id: id.to_owned(),
        name: name.to_owned(),
        handle: None,
        avatar: Vec::new(),
        subscriber_count: None,
        verified: false,
    }
}

fn video(id: &str, day: i64, is_short: bool) -> ChannelVideoUpsert {
    ChannelVideoUpsert {
        video_id: id.to_owned(),
        title: format!("Video {id}"),
        duration_seconds: Some(if is_short { 30 } else { 300 }),
        published_at: Some(datetime!(2026-01-01 00:00 UTC) + time::Duration::days(day)),
        published_text: None,
        thumbnails: Vec::new(),
        view_count: None,
        is_live: false,
        is_upcoming: false,
        is_short,
    }
}

/// Subscribes a new user to two channels: A with videos a1, a2 and short s1, B with b1.
async fn seeded(pool: &PgPool) -> User {
    let subs = PgSubscriptionRepository::new(pool.clone());
    let channels = PgChannelRepository::new(pool.clone());
    let alice = user(pool).await;
    subs.subscribe(alice.id, CHANNEL_A).await.unwrap();
    subs.subscribe(alice.id, CHANNEL_B).await.unwrap();
    channels
        .store_fetch(
            &channel(CHANNEL_A, "Alpha"),
            &[
                video("aaaaaaaaaa1", 1, false),
                video("aaaaaaaaaa2", 2, false),
                video("aaaaaaaaas1", 3, true),
            ],
        )
        .await
        .unwrap();
    channels
        .store_fetch(
            &channel(CHANNEL_B, "Beta"),
            &[video("bbbbbbbbbb1", 4, false)],
        )
        .await
        .unwrap();
    alice
}

async fn feed_ids(pool: &PgPool, user: &User, filter: &FeedFilter) -> Vec<String> {
    PgSubscriptionRepository::new(pool.clone())
        .feed(user.id, filter, None, 50)
        .await
        .unwrap()
        .into_iter()
        .map(|v| v.video_id)
        .collect()
}

#[tokio::test]
#[ignore = "requires Docker or OXISTREAM_TEST_DATABASE_URL"]
async fn feed_filters_watched_channel_and_hidden() {
    let db = isolated_test_pool().await;
    let alice = seeded(&db.pool).await;
    let history = PgHistoryRepository::new(db.pool.clone());
    let hidden = PgHiddenVideoRepository::new(db.pool.clone());

    let all = FeedFilter::from(FeedKind::Videos);
    assert_eq!(
        feed_ids(&db.pool, &alice, &all).await,
        ["bbbbbbbbbb1", "aaaaaaaaaa2", "aaaaaaaaaa1"]
    );

    // Started but unfinished videos stay; finished ones go.
    history
        .record(
            alice.id,
            &ProgressUpdate {
                video_id: "aaaaaaaaaa1".into(),
                position_seconds: 100.0,
                duration_seconds: 300.0,
                finished: false,
            },
        )
        .await
        .unwrap();
    history
        .mark_watched(alice.id, "aaaaaaaaaa2", None)
        .await
        .unwrap()
        .unwrap();
    let unwatched = FeedFilter {
        hide_watched: true,
        ..all.clone()
    };
    assert_eq!(
        feed_ids(&db.pool, &alice, &unwatched).await,
        ["bbbbbbbbbb1", "aaaaaaaaaa1"]
    );
    assert_eq!(feed_ids(&db.pool, &alice, &all).await.len(), 3);

    let only_a = FeedFilter {
        channel_id: Some(CHANNEL_A.into()),
        kind: FeedKind::All,
        ..FeedFilter::default()
    };
    assert_eq!(
        feed_ids(&db.pool, &alice, &only_a).await,
        ["aaaaaaaaas1", "aaaaaaaaaa2", "aaaaaaaaaa1"]
    );

    assert!(hidden.hide(alice.id, "bbbbbbbbbb1").await.unwrap());
    assert!(!hidden.hide(alice.id, "bbbbbbbbbb1").await.unwrap());
    assert_eq!(
        feed_ids(&db.pool, &alice, &all).await,
        ["aaaaaaaaaa2", "aaaaaaaaaa1"]
    );
    let bob = seeded(&db.pool).await;
    assert_eq!(
        feed_ids(&db.pool, &bob, &all).await.len(),
        3,
        "hidden per user"
    );
    assert_eq!(
        hidden.hidden_ids(alice.id).await.unwrap(),
        ["bbbbbbbbbb1".to_owned()].into()
    );
    assert!(hidden.unhide(alice.id, "bbbbbbbbbb1").await.unwrap());
    assert!(!hidden.unhide(alice.id, "bbbbbbbbbb1").await.unwrap());
    assert_eq!(feed_ids(&db.pool, &alice, &all).await.len(), 3);
}

#[tokio::test]
#[ignore = "requires Docker or OXISTREAM_TEST_DATABASE_URL"]
async fn mark_watched_finishes_videos_but_not_shorts() {
    let db = isolated_test_pool().await;
    let alice = seeded(&db.pool).await;
    let history = PgHistoryRepository::new(db.pool.clone());
    let settings = PgUserSettingsRepository::new(db.pool.clone());
    let hidden = PgHiddenVideoRepository::new(db.pool.clone());

    // Duration from the feed's metadata.
    let progress = history
        .mark_watched(alice.id, "aaaaaaaaaa1", None)
        .await
        .unwrap()
        .unwrap();
    assert!(progress.finished);
    assert_eq!(progress.position_seconds, 300.0);
    assert_eq!(progress.duration_seconds, 300.0);

    assert_eq!(
        history
            .mark_watched(alice.id, "aaaaaaaaas1", Some(30.0))
            .await
            .unwrap(),
        None,
        "shorts have no history"
    );
    assert_eq!(
        history
            .mark_watched(alice.id, "zzzzzzzzzzz", None)
            .await
            .unwrap(),
        None,
        "unknown duration"
    );
    let unknown = history
        .mark_watched(alice.id, "zzzzzzzzzzz", Some(120.0))
        .await
        .unwrap()
        .unwrap();
    assert_eq!(unknown.duration_seconds, 120.0);

    // An existing entry keeps the player's duration and leaves "Continue watching".
    history
        .record(
            alice.id,
            &ProgressUpdate {
                video_id: "bbbbbbbbbb1".into(),
                position_seconds: 50.0,
                duration_seconds: 301.5,
                finished: false,
            },
        )
        .await
        .unwrap();
    assert_eq!(
        history.unfinished(alice.id, 10.0, 10).await.unwrap().len(),
        1
    );
    settings.set_history_paused(alice.id, true).await.unwrap();
    let progress = history
        .mark_watched(alice.id, "bbbbbbbbbb1", Some(999.0))
        .await
        .unwrap()
        .expect("recorded while paused");
    assert_eq!(progress.position_seconds, 301.5);
    assert_eq!(
        history.unfinished(alice.id, 10.0, 10).await.unwrap().len(),
        0
    );
    let list = history.list(alice.id, None, None, 10).await.unwrap();
    assert_eq!(list[0].progress.video_id, "bbbbbbbbbb1");

    // Hidden videos leave "Continue watching", not the history.
    settings.set_history_paused(alice.id, false).await.unwrap();
    history
        .record(
            alice.id,
            &ProgressUpdate {
                video_id: "aaaaaaaaaa2".into(),
                position_seconds: 50.0,
                duration_seconds: 300.0,
                finished: false,
            },
        )
        .await
        .unwrap();
    assert_eq!(
        history.unfinished(alice.id, 10.0, 10).await.unwrap().len(),
        1
    );
    hidden.hide(alice.id, "aaaaaaaaaa2").await.unwrap();
    assert_eq!(
        history.unfinished(alice.id, 10.0, 10).await.unwrap().len(),
        0
    );
    assert_eq!(
        history.list(alice.id, None, None, 10).await.unwrap().len(),
        4
    );
}

#[tokio::test]
#[ignore = "requires Docker or OXISTREAM_TEST_DATABASE_URL"]
async fn feed_settings_are_stored() {
    let db = isolated_test_pool().await;
    let settings = PgUserSettingsRepository::new(db.pool.clone());
    let alice = user(&db.pool).await;
    let initial = settings.get(alice.id).await.unwrap();
    assert!(!initial.feed_hide_watched && !initial.feed_hide_shorts);
    let updated = settings
        .update(
            alice.id,
            &oxistream_db::SettingsUpdate {
                feed_hide_watched: Some(true),
                ..Default::default()
            },
        )
        .await
        .unwrap();
    assert!(updated.feed_hide_watched && !updated.feed_hide_shorts);
    let updated = settings
        .update(
            alice.id,
            &oxistream_db::SettingsUpdate {
                feed_hide_shorts: Some(true),
                ..Default::default()
            },
        )
        .await
        .unwrap();
    assert!(updated.feed_hide_watched && updated.feed_hide_shorts);
}

struct Api {
    app: Router,
    token: String,
}

impl Api {
    async fn new(pool: &PgPool, user: &User) -> Self {
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
        Self {
            app: app_with_pool(pool.clone()),
            token,
        }
    }

    async fn send(
        &self,
        method: Method,
        uri: &str,
        signed_in: bool,
        body: Option<Value>,
    ) -> (StatusCode, Value) {
        let mut req = Request::builder()
            .method(method)
            .uri(uri)
            .header(header::HOST, "tube.test")
            .header(header::ORIGIN, "http://tube.test");
        if signed_in {
            req = req.header(header::COOKIE, format!("oxistream_session={}", self.token));
        }
        let body = match body {
            Some(json) => {
                req = req.header(header::CONTENT_TYPE, "application/json");
                Body::from(json.to_string())
            }
            None => Body::empty(),
        };
        let response = self
            .app
            .clone()
            .oneshot(req.body(body).unwrap())
            .await
            .unwrap();
        let status = response.status();
        let bytes = response.into_body().collect().await.unwrap().to_bytes();
        (
            status,
            serde_json::from_slice(&bytes).unwrap_or(Value::Null),
        )
    }

    async fn feed(&self, query: &str) -> Vec<String> {
        let (status, page) = self
            .send(Method::GET, &format!("/api/v0/feed?{query}"), true, None)
            .await;
        assert_eq!(status, StatusCode::OK, "{query}: {page}");
        page["items"]
            .as_array()
            .unwrap()
            .iter()
            .map(|v| v["id"].as_str().unwrap().to_owned())
            .collect()
    }
}

#[tokio::test]
#[ignore = "requires Docker or OXISTREAM_TEST_DATABASE_URL"]
async fn api_filters_feed_and_marks_videos() {
    let db = isolated_test_pool().await;
    let alice = seeded(&db.pool).await;
    let api = Api::new(&db.pool, &alice).await;

    for (method, uri) in [
        (Method::PUT, "/api/v0/hidden-videos/aaaaaaaaaa1"),
        (Method::DELETE, "/api/v0/hidden-videos/aaaaaaaaaa1"),
        (Method::PUT, "/api/v0/history/aaaaaaaaaa1"),
    ] {
        let (status, _) = api.send(method.clone(), uri, false, Some(json!({}))).await;
        assert_eq!(status, StatusCode::UNAUTHORIZED, "{method} {uri}");
    }

    let (status, progress) = api
        .send(
            Method::PUT,
            "/api/v0/history/aaaaaaaaaa2",
            true,
            Some(json!({})),
        )
        .await;
    assert_eq!(status, StatusCode::OK, "{progress}");
    assert_eq!(progress["finished"], true);
    assert_eq!(progress["position_seconds"], 300.0);
    assert_eq!(
        api.feed("kind=videos&hide_watched=true").await,
        ["bbbbbbbbbb1", "aaaaaaaaaa1"]
    );
    assert_eq!(api.feed("kind=videos").await.len(), 3);
    assert_eq!(
        api.feed(&format!("channel={CHANNEL_B}")).await,
        ["bbbbbbbbbb1"]
    );

    let (status, _) = api
        .send(Method::DELETE, "/api/v0/history/aaaaaaaaaa2", true, None)
        .await;
    assert_eq!(status, StatusCode::NO_CONTENT, "unwatched");
    assert_eq!(api.feed("kind=videos&hide_watched=true").await.len(), 3);

    let (status, _) = api
        .send(Method::PUT, "/api/v0/hidden-videos/bbbbbbbbbb1", true, None)
        .await;
    assert_eq!(status, StatusCode::NO_CONTENT);
    assert_eq!(
        api.feed("kind=videos").await,
        ["aaaaaaaaaa2", "aaaaaaaaaa1"]
    );
    let (status, _) = api
        .send(
            Method::DELETE,
            "/api/v0/hidden-videos/bbbbbbbbbb1",
            true,
            None,
        )
        .await;
    assert_eq!(status, StatusCode::NO_CONTENT);
    assert_eq!(api.feed("kind=videos").await.len(), 3);

    let (status, settings) = api
        .send(
            Method::PATCH,
            "/api/v0/settings",
            true,
            Some(json!({ "feed_hide_watched": true, "feed_hide_shorts": true })),
        )
        .await;
    assert_eq!(status, StatusCode::OK, "{settings}");
    assert_eq!(settings["feed_hide_watched"], true);
    assert_eq!(settings["feed_hide_shorts"], true);
}

#[tokio::test]
#[ignore = "requires Docker or OXISTREAM_TEST_DATABASE_URL"]
async fn api_rejects_bad_filter_and_mark_input() {
    let db = isolated_test_pool().await;
    let alice = seeded(&db.pool).await;
    let api = Api::new(&db.pool, &alice).await;
    for (method, uri, body) in [
        (Method::GET, "/api/v0/feed?channel=nope", None),
        (Method::GET, "/api/v0/feed?hide_watched=maybe", None),
        (Method::PUT, "/api/v0/hidden-videos/nope", None),
        (Method::DELETE, "/api/v0/hidden-videos/nope", None),
        (Method::PUT, "/api/v0/history/nope", Some(json!({}))),
        (
            Method::PUT,
            "/api/v0/history/aaaaaaaaaa1",
            Some(json!({ "duration_seconds": -1 })),
        ),
    ] {
        let (status, json) = api.send(method.clone(), uri, true, body).await;
        assert_eq!(status, StatusCode::BAD_REQUEST, "{method} {uri}: {json}");
    }
    for id in ["aaaaaaaaas1", "zzzzzzzzzzz"] {
        let (status, json) = api
            .send(
                Method::PUT,
                &format!("/api/v0/history/{id}"),
                true,
                Some(json!({})),
            )
            .await;
        assert_eq!(status, StatusCode::UNPROCESSABLE_ENTITY, "{id}: {json}");
    }
}
