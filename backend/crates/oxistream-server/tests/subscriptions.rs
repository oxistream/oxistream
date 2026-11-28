//! Needs `OXISTREAM_TEST_DATABASE_URL` or a Docker daemon; run with `cargo test -- --include-ignored`.

mod common;

use std::collections::HashMap;
use std::sync::{Arc, Mutex};
use std::time::Duration;

use async_trait::async_trait;
use axum::Router;
use axum::body::Body;
use axum::http::{Method, Request, StatusCode, header};
use common::{isolated_test_pool, state_with_pool, unique_suffix};
use http_body_util::BodyExt;
use oxistream_db::{
    AdvisoryLock, ChannelRepository, ChannelUpdate, ChannelVideoUpsert, FeedCursor, FeedKind,
    Image, LockKey, NewSession, NewUser, PgChannelRepository, PgPool, PgSessionRepository,
    PgSubscriptionRepository, PgUserRepository, SessionRepository, SubscriptionRepository, User,
    UserRepository,
};
use oxistream_server::AppState;
use oxistream_server::auth::cookie;
use oxistream_server::feed::{ChannelFetch, ChannelSource, Feed, FeedWorker, RunSummary};
use oxistream_server::youtube::{self, YouTubeError};
use serde_json::Value;
use time::OffsetDateTime;
use time::macros::datetime;
use tower::ServiceExt;

const CHANNEL_A: &str = "UCaaaaaaaaaaaaaaaaaaaaaa";
const CHANNEL_B: &str = "UCbbbbbbbbbbbbbbbbbbbbbb";
const CHANNEL_C: &str = "UCcccccccccccccccccccccc";
const INTERVAL: Duration = Duration::from_mins(30);

/// The worker lock is per database, not per schema, so worker tests take turns.
static WORKER_LOCK: tokio::sync::Mutex<()> = tokio::sync::Mutex::const_new(());

async fn user(pool: &PgPool) -> User {
    PgUserRepository::new(pool.clone())
        .create(&NewUser {
            username: format!("u_{}", unique_suffix()),
            password_hash: None,
        })
        .await
        .unwrap()
}

fn image(url: &str) -> Image {
    Image {
        url: url.to_owned(),
        width: 320,
        height: 180,
    }
}

fn channel(id: &str, name: &str) -> ChannelUpdate {
    ChannelUpdate {
        id: id.to_owned(),
        name: name.to_owned(),
        handle: Some(format!("@{name}")),
        avatar: vec![image("https://yt3.ggpht.com/avatar")],
        subscriber_count: Some(1000),
        verified: true,
    }
}

fn video(id: &str, published_at: OffsetDateTime) -> ChannelVideoUpsert {
    ChannelVideoUpsert {
        video_id: id.to_owned(),
        title: format!("Video {id}"),
        duration_seconds: Some(213),
        published_at: Some(published_at),
        published_text: Some("1 day ago".into()),
        thumbnails: vec![image(&format!("https://i.ytimg.com/vi/{id}/hqdefault.jpg"))],
        view_count: Some(42),
        is_live: false,
        is_upcoming: false,
        is_short: false,
    }
}

#[tokio::test]
#[ignore = "requires Docker or OXISTREAM_TEST_DATABASE_URL"]
async fn subscribe_list_and_unsubscribe() {
    let db = isolated_test_pool().await;
    let subs = PgSubscriptionRepository::new(db.pool.clone());
    let channels = PgChannelRepository::new(db.pool.clone());
    let alice = user(&db.pool).await;

    let (sub, created) = subs.subscribe(alice.id, CHANNEL_B).await.unwrap();
    assert!(created);
    assert_eq!(sub.channel.id, CHANNEL_B);
    assert_eq!(sub.channel.name, None, "unknown until fetched");
    let (_, created) = subs.subscribe(alice.id, CHANNEL_B).await.unwrap();
    assert!(!created, "subscribing is idempotent");
    subs.subscribe(alice.id, CHANNEL_A).await.unwrap();
    channels
        .store_fetch(&channel(CHANNEL_B, "Bravo"), &[])
        .await
        .unwrap();
    channels
        .store_fetch(&channel(CHANNEL_A, "alpha"), &[])
        .await
        .unwrap();

    let list = subs.list(alice.id).await.unwrap();
    let names: Vec<_> = list.iter().map(|s| s.channel.name.as_deref()).collect();
    assert_eq!(
        names,
        [Some("alpha"), Some("Bravo")],
        "case-insensitive order"
    );
    assert_eq!(list[1].channel.handle.as_deref(), Some("@Bravo"));
    assert_eq!(list[1].channel.avatar.len(), 1);
    assert!(list[1].channel.fetched_at.is_some());

    assert!(subs.is_subscribed(alice.id, CHANNEL_A).await.unwrap());
    assert!(subs.unsubscribe(alice.id, CHANNEL_A).await.unwrap());
    assert!(!subs.unsubscribe(alice.id, CHANNEL_A).await.unwrap());
    assert!(!subs.is_subscribed(alice.id, CHANNEL_A).await.unwrap());
    assert_eq!(subs.list(alice.id).await.unwrap().len(), 1);
}

#[tokio::test]
#[ignore = "requires Docker or OXISTREAM_TEST_DATABASE_URL"]
async fn feed_is_newest_first_paginated_and_per_user() {
    let db = isolated_test_pool().await;
    let subs = PgSubscriptionRepository::new(db.pool.clone());
    let channels = PgChannelRepository::new(db.pool.clone());
    let alice = user(&db.pool).await;
    let bob = user(&db.pool).await;
    subs.subscribe(alice.id, CHANNEL_A).await.unwrap();
    subs.subscribe(alice.id, CHANNEL_B).await.unwrap();
    subs.subscribe(bob.id, CHANNEL_C).await.unwrap();

    channels
        .store_fetch(
            &channel(CHANNEL_A, "Alpha"),
            &[
                video("aaaaaaaaaa1", datetime!(2026-01-01 00:00 UTC)),
                video("aaaaaaaaaa2", datetime!(2026-01-03 00:00 UTC)),
            ],
        )
        .await
        .unwrap();
    channels
        .store_fetch(
            &channel(CHANNEL_B, "Bravo"),
            &[video("bbbbbbbbbb1", datetime!(2026-01-02 00:00 UTC))],
        )
        .await
        .unwrap();
    channels
        .store_fetch(
            &channel(CHANNEL_C, "Charlie"),
            &[video("cccccccccc1", datetime!(2026-01-04 00:00 UTC))],
        )
        .await
        .unwrap();

    let first = subs
        .feed(alice.id, &FeedKind::All.into(), None, 2)
        .await
        .unwrap();
    let ids: Vec<_> = first.iter().map(|v| v.video_id.as_str()).collect();
    assert_eq!(ids, ["aaaaaaaaaa2", "bbbbbbbbbb1"]);
    assert_eq!(first[1].channel_name.as_deref(), Some("Bravo"));
    assert!(first[1].channel_verified);
    assert_eq!(first[1].thumbnails.len(), 1);

    let rest = subs
        .feed(alice.id, &FeedKind::All.into(), Some(&first[1].cursor()), 2)
        .await
        .unwrap();
    let ids: Vec<_> = rest.iter().map(|v| v.video_id.as_str()).collect();
    assert_eq!(ids, ["aaaaaaaaaa1"]);

    let bobs = subs
        .feed(bob.id, &FeedKind::All.into(), None, 10)
        .await
        .unwrap();
    assert_eq!(bobs.len(), 1);
    assert_eq!(bobs[0].video_id, "cccccccccc1");

    let cursor = FeedCursor {
        sort_at: datetime!(2026-01-02 00:00 UTC),
        video_id: "bbbbbbbbbb1".into(),
    };
    let after = subs
        .feed(alice.id, &FeedKind::All.into(), Some(&cursor), 10)
        .await
        .unwrap();
    assert_eq!(after.len(), 1, "the cursor itself is excluded");
}

#[tokio::test]
#[ignore = "requires Docker or OXISTREAM_TEST_DATABASE_URL"]
async fn feed_filters_shorts_and_keeps_their_channel_order() {
    let db = isolated_test_pool().await;
    let subs = PgSubscriptionRepository::new(db.pool.clone());
    let channels = PgChannelRepository::new(db.pool.clone());
    let alice = user(&db.pool).await;
    subs.subscribe(alice.id, CHANNEL_A).await.unwrap();
    let short = |id: &str| ChannelVideoUpsert {
        published_at: None,
        published_text: None,
        is_short: true,
        ..video(id, datetime!(2026-01-01 00:00 UTC))
    };
    channels
        .store_fetch(
            &channel(CHANNEL_A, "Alpha"),
            &[
                video("aaaaaaaaaa1", datetime!(2026-01-01 00:00 UTC)),
                short("aaaaaaaaas1"),
                short("aaaaaaaaas0"),
                short("aaaaaaaaas2"),
            ],
        )
        .await
        .unwrap();

    let ids = |videos: Vec<oxistream_db::FeedVideo>| {
        videos.into_iter().map(|v| v.video_id).collect::<Vec<_>>()
    };
    let shorts = subs
        .feed(alice.id, &FeedKind::Shorts.into(), None, 10)
        .await
        .unwrap();
    assert_eq!(
        ids(shorts),
        ["aaaaaaaaas1", "aaaaaaaaas0", "aaaaaaaaas2"],
        "undated shorts keep the channel's order"
    );
    let videos = subs
        .feed(alice.id, &FeedKind::Videos.into(), None, 10)
        .await
        .unwrap();
    assert_eq!(ids(videos), ["aaaaaaaaaa1"]);
    let all = subs
        .feed(alice.id, &FeedKind::All.into(), None, 10)
        .await
        .unwrap();
    assert_eq!(all.len(), 4);
}

#[tokio::test]
#[ignore = "requires Docker or OXISTREAM_TEST_DATABASE_URL"]
async fn refetch_keeps_publish_date_and_owner() {
    let db = isolated_test_pool().await;
    let subs = PgSubscriptionRepository::new(db.pool.clone());
    let channels = PgChannelRepository::new(db.pool.clone());
    let alice = user(&db.pool).await;
    subs.subscribe(alice.id, CHANNEL_A).await.unwrap();
    subs.subscribe(alice.id, CHANNEL_B).await.unwrap();

    let original = datetime!(2026-01-01 10:00 UTC);
    channels
        .store_fetch(
            &channel(CHANNEL_A, "Alpha"),
            &[video("aaaaaaaaaa1", original)],
        )
        .await
        .unwrap();
    channels
        .store_fetch(
            &channel(CHANNEL_A, "Alpha renamed"),
            &[ChannelVideoUpsert {
                title: "New title".into(),
                view_count: Some(1000),
                ..video("aaaaaaaaaa1", datetime!(2026-01-02 00:00 UTC))
            }],
        )
        .await
        .unwrap();
    // Seen on another channel's page, e.g. a collaboration; stays with its first channel.
    channels
        .store_fetch(
            &channel(CHANNEL_B, "Bravo"),
            &[ChannelVideoUpsert {
                title: "Hijacked".into(),
                ..video("aaaaaaaaaa1", original)
            }],
        )
        .await
        .unwrap();

    let feed = subs
        .feed(alice.id, &FeedKind::All.into(), None, 10)
        .await
        .unwrap();
    assert_eq!(feed.len(), 1);
    assert_eq!(feed[0].channel_id, CHANNEL_A);
    assert_eq!(feed[0].channel_name.as_deref(), Some("Alpha renamed"));
    assert_eq!(feed[0].title, "New title");
    assert_eq!(feed[0].view_count, Some(1000));
    assert_eq!(feed[0].published_at, Some(original));
}

async fn set_last_visit(pool: &PgPool, channel_id: &str, at: OffsetDateTime) {
    sqlx::query("UPDATE subscriptions SET last_visited_at = $2 WHERE channel_id = $1")
        .bind(channel_id)
        .bind(at)
        .execute(pool)
        .await
        .unwrap();
}

async fn has_new(subs: &PgSubscriptionRepository, user_id: oxistream_db::Uuid) -> Vec<bool> {
    subs.list(user_id)
        .await
        .unwrap()
        .iter()
        .map(|s| s.has_new)
        .collect()
}

#[tokio::test]
#[ignore = "requires Docker or OXISTREAM_TEST_DATABASE_URL"]
async fn new_uploads_since_the_last_visit() {
    let db = isolated_test_pool().await;
    let subs = PgSubscriptionRepository::new(db.pool.clone());
    let channels = PgChannelRepository::new(db.pool.clone());
    let alice = user(&db.pool).await;
    subs.subscribe(alice.id, CHANNEL_A).await.unwrap();
    subs.subscribe(alice.id, CHANNEL_B).await.unwrap();
    let short = |id: &str| ChannelVideoUpsert {
        published_at: None,
        published_text: None,
        is_short: true,
        ..video(id, datetime!(2026-01-01 00:00 UTC))
    };

    channels
        .store_fetch(
            &channel(CHANNEL_A, "Alpha"),
            &[
                video("aaaaaaaaaa1", datetime!(2026-01-02 00:00 UTC)),
                short("aaaaaaaaas1"),
            ],
        )
        .await
        .unwrap();
    channels
        .store_fetch(&channel(CHANNEL_B, "Bravo"), &[short("bbbbbbbbbs1")])
        .await
        .unwrap();
    assert_eq!(
        has_new(&subs, alice.id).await,
        [false, false],
        "nothing of the first fetch is new"
    );

    set_last_visit(&db.pool, CHANNEL_A, datetime!(2026-01-01 12:00 UTC)).await;
    assert_eq!(has_new(&subs, alice.id).await, [true, false]);

    channels
        .store_fetch(
            &channel(CHANNEL_B, "Bravo"),
            &[short("bbbbbbbbbs2"), short("bbbbbbbbbs1")],
        )
        .await
        .unwrap();
    assert_eq!(
        has_new(&subs, alice.id).await,
        [true, true],
        "a short seen after the visit"
    );

    let bob = user(&db.pool).await;
    subs.subscribe(bob.id, CHANNEL_A).await.unwrap();
    assert_eq!(has_new(&subs, bob.id).await, [false], "new subscription");

    for id in ["aaaaaaaaaa1", "aaaaaaaaas1"] {
        sqlx::query("INSERT INTO hidden_videos (user_id, video_id) VALUES ($1, $2)")
            .bind(alice.id)
            .bind(id)
            .execute(&db.pool)
            .await
            .unwrap();
    }
    assert_eq!(has_new(&subs, alice.id).await, [false, true], "hidden");

    assert!(subs.visit(alice.id, CHANNEL_B).await.unwrap());
    assert_eq!(has_new(&subs, alice.id).await, [false, false]);
    assert!(
        !subs.visit(bob.id, CHANNEL_B).await.unwrap(),
        "not subscribed"
    );

    channels
        .store_fetch(
            &channel(CHANNEL_B, "Bravo"),
            &[ChannelVideoUpsert {
                published_at: Some(OffsetDateTime::now_utc() + time::Duration::days(1)),
                is_upcoming: true,
                ..video("bbbbbbbbbu1", datetime!(2026-01-01 00:00 UTC))
            }],
        )
        .await
        .unwrap();
    assert_eq!(
        has_new(&subs, alice.id).await,
        [false, false],
        "upcoming videos are not uploads"
    );
}

#[derive(Default)]
struct FakeSource {
    fetches: Mutex<HashMap<String, usize>>,
    failures: HashMap<String, YouTubeError>,
}

impl FakeSource {
    fn failing(failures: &[(&str, YouTubeError)]) -> Self {
        Self {
            failures: failures
                .iter()
                .map(|(id, err)| ((*id).to_owned(), err.clone()))
                .collect(),
            ..Self::default()
        }
    }

    fn fetches(&self, id: &str) -> usize {
        self.fetches.lock().unwrap().get(id).copied().unwrap_or(0)
    }

    fn total(&self) -> usize {
        self.fetches.lock().unwrap().values().sum()
    }
}

#[async_trait]
impl ChannelSource for FakeSource {
    async fn fetch(&self, channel_id: &str) -> youtube::Result<ChannelFetch> {
        *self
            .fetches
            .lock()
            .unwrap()
            .entry(channel_id.to_owned())
            .or_default() += 1;
        if let Some(err) = self.failures.get(channel_id) {
            return Err(err.clone());
        }
        let prefix = &channel_id[2..12];
        Ok(ChannelFetch {
            channel: channel(channel_id, &format!("Channel {prefix}")),
            videos: vec![video(
                &format!("{}v", &prefix[..10]),
                datetime!(2026-01-01 00:00 UTC),
            )],
        })
    }
}

fn worker(pool: &PgPool, source: &Arc<FakeSource>) -> FeedWorker {
    FeedWorker::new(
        pool.clone(),
        Arc::new(PgChannelRepository::new(pool.clone())),
        source.clone(),
        INTERVAL,
    )
}

#[tokio::test]
#[ignore = "requires Docker or OXISTREAM_TEST_DATABASE_URL"]
async fn worker_fetches_each_subscribed_channel_once() {
    let _turn = WORKER_LOCK.lock().await;
    let db = isolated_test_pool().await;
    let subs = PgSubscriptionRepository::new(db.pool.clone());
    let channels = PgChannelRepository::new(db.pool.clone());
    let (alice, bob) = (user(&db.pool).await, user(&db.pool).await);
    subs.subscribe(alice.id, CHANNEL_A).await.unwrap();
    subs.subscribe(bob.id, CHANNEL_A).await.unwrap();
    subs.subscribe(bob.id, CHANNEL_B).await.unwrap();
    subs.subscribe(bob.id, CHANNEL_C).await.unwrap();
    let source = Arc::new(FakeSource::failing(&[(
        CHANNEL_C,
        YouTubeError::NotFound("gone".into()),
    )]));
    let worker = worker(&db.pool, &source);

    let summary = worker.run_once().await.unwrap().expect("lock is free");
    assert_eq!(
        summary,
        RunSummary {
            fetched: 2,
            failed: 1,
            pruned: 0,
            rate_limited: false,
            ..RunSummary::default()
        }
    );
    assert_eq!(source.fetches(CHANNEL_A), 1, "once despite two subscribers");
    assert_eq!(source.fetches(CHANNEL_B), 1);
    assert_eq!(
        subs.feed(alice.id, &FeedKind::All.into(), None, 10)
            .await
            .unwrap()
            .len(),
        1
    );
    assert_eq!(
        subs.feed(bob.id, &FeedKind::All.into(), None, 10)
            .await
            .unwrap()
            .len(),
        2
    );

    let failed = channels.get(CHANNEL_C).await.unwrap().unwrap();
    assert!(failed.last_error.unwrap().contains("gone"));
    assert!(failed.fetched_at.is_none());
    let status = channels.status().await.unwrap();
    assert_eq!((status.channels, status.pending, status.failing), (3, 1, 1));
    assert!(status.last_checked_at.is_some());

    let again = worker.run_once().await.unwrap().unwrap();
    assert_eq!(again, RunSummary::default(), "nothing is due yet");
    assert_eq!(source.total(), 3);
}

#[tokio::test]
#[ignore = "requires Docker or OXISTREAM_TEST_DATABASE_URL"]
async fn worker_prunes_unsubscribed_channels() {
    let _turn = WORKER_LOCK.lock().await;
    let db = isolated_test_pool().await;
    let subs = PgSubscriptionRepository::new(db.pool.clone());
    let channels = PgChannelRepository::new(db.pool.clone());
    let alice = user(&db.pool).await;
    subs.subscribe(alice.id, CHANNEL_A).await.unwrap();
    let source = Arc::new(FakeSource::default());
    let worker = worker(&db.pool, &source);
    worker.run_once().await.unwrap().unwrap();
    assert_eq!(
        subs.feed(alice.id, &FeedKind::All.into(), None, 10)
            .await
            .unwrap()
            .len(),
        1
    );

    subs.unsubscribe(alice.id, CHANNEL_A).await.unwrap();
    let summary = worker.run_once().await.unwrap().unwrap();
    assert_eq!(summary.pruned, 1);
    assert_eq!(summary.fetched, 0, "unsubscribed channels are not fetched");
    assert_eq!(channels.status().await.unwrap().channels, 0);

    subs.subscribe(alice.id, CHANNEL_A).await.unwrap();
    assert!(
        worker.fetch_requested(CHANNEL_A).await.unwrap(),
        "a resubscribed channel is fetched again"
    );
    assert_eq!(source.fetches(CHANNEL_A), 2);
}

#[tokio::test]
#[ignore = "requires Docker or OXISTREAM_TEST_DATABASE_URL"]
async fn worker_stops_when_rate_limited() {
    let _turn = WORKER_LOCK.lock().await;
    let db = isolated_test_pool().await;
    let subs = PgSubscriptionRepository::new(db.pool.clone());
    let channels = PgChannelRepository::new(db.pool.clone());
    let alice = user(&db.pool).await;
    for id in [CHANNEL_A, CHANNEL_B, CHANNEL_C] {
        subs.subscribe(alice.id, id).await.unwrap();
    }
    let source = Arc::new(FakeSource::failing(&[(
        CHANNEL_A,
        YouTubeError::RateLimited("captcha".into()),
    )]));
    let summary = worker(&db.pool, &source).run_once().await.unwrap().unwrap();
    assert!(summary.rate_limited);
    assert_eq!(source.total(), 1, "the run stops at the first block");
    let channel = channels.get(CHANNEL_A).await.unwrap().unwrap();
    assert_eq!(channel.checked_at, None, "retried on the next run");
}

#[tokio::test]
#[ignore = "requires Docker or OXISTREAM_TEST_DATABASE_URL"]
async fn worker_skips_while_another_replica_holds_the_lock() {
    let _turn = WORKER_LOCK.lock().await;
    let db = isolated_test_pool().await;
    let subs = PgSubscriptionRepository::new(db.pool.clone());
    let alice = user(&db.pool).await;
    subs.subscribe(alice.id, CHANNEL_A).await.unwrap();
    let source = Arc::new(FakeSource::default());
    let worker = worker(&db.pool, &source);

    let held = AdvisoryLock::try_acquire(&db.pool, LockKey::FeedWorker)
        .await
        .unwrap()
        .expect("lock is free");
    assert!(
        AdvisoryLock::try_acquire(&db.pool, LockKey::FeedWorker)
            .await
            .unwrap()
            .is_none()
    );
    assert_eq!(worker.run_once().await.unwrap(), None);
    assert_eq!(source.total(), 0);

    held.release().await.unwrap();
    assert_eq!(worker.run_once().await.unwrap().unwrap().fetched, 1);
}

#[tokio::test]
#[ignore = "requires Docker or OXISTREAM_TEST_DATABASE_URL"]
async fn requested_fetch_skips_fresh_channels() {
    let db = isolated_test_pool().await;
    let subs = PgSubscriptionRepository::new(db.pool.clone());
    let alice = user(&db.pool).await;
    subs.subscribe(alice.id, CHANNEL_A).await.unwrap();
    let source = Arc::new(FakeSource::default());
    let worker = worker(&db.pool, &source);

    assert!(worker.fetch_requested(CHANNEL_A).await.unwrap());
    assert!(!worker.fetch_requested(CHANNEL_A).await.unwrap());
    assert_eq!(source.fetches(CHANNEL_A), 1);
}

struct Api {
    app: Router,
    token: String,
}

impl Api {
    async fn new(pool: &PgPool, feed: Feed) -> Self {
        let user = user(pool).await;
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
        let app = oxistream_server::app(AppState {
            feed,
            ..state_with_pool(pool.clone())
        });
        Self { app, token }
    }

    async fn send(&self, method: Method, uri: &str, signed_in: bool) -> (StatusCode, Value) {
        let (status, _, bytes) = self.request(method, uri, signed_in, Body::empty()).await;
        (
            status,
            serde_json::from_slice(&bytes).unwrap_or(Value::Null),
        )
    }

    async fn import(&self, body: impl Into<Body>) -> (StatusCode, Value) {
        let (status, _, bytes) = self
            .request(
                Method::POST,
                "/api/v0/subscriptions/import",
                true,
                body.into(),
            )
            .await;
        (
            status,
            serde_json::from_slice(&bytes).unwrap_or(Value::Null),
        )
    }

    async fn request(
        &self,
        method: Method,
        uri: &str,
        signed_in: bool,
        body: Body,
    ) -> (StatusCode, header::HeaderMap, Vec<u8>) {
        let mut req = Request::builder()
            .method(method)
            .uri(uri)
            .header(header::HOST, "tube.test")
            .header(header::ORIGIN, "http://tube.test");
        if signed_in {
            req = req.header(header::COOKIE, format!("oxistream_session={}", self.token));
        }
        let response = self
            .app
            .clone()
            .oneshot(req.body(body).unwrap())
            .await
            .unwrap();
        let status = response.status();
        let headers = response.headers().clone();
        let bytes = response.into_body().collect().await.unwrap().to_bytes();
        (status, headers, bytes.to_vec())
    }
}

fn fixture(name: &str) -> Vec<u8> {
    let path = format!(
        "{}/tests/fixtures/subscriptions/{name}",
        env!("CARGO_MANIFEST_DIR")
    );
    std::fs::read(&path).unwrap_or_else(|err| panic!("{path}: {err}"))
}

#[tokio::test]
#[ignore = "requires Docker or OXISTREAM_TEST_DATABASE_URL"]
async fn api_requires_login() {
    let db = isolated_test_pool().await;
    let channels = Arc::new(PgChannelRepository::new(db.pool.clone()));
    let api = Api::new(&db.pool, Feed::new(channels).0).await;
    let path = format!("/api/v0/subscriptions/{CHANNEL_A}");
    let visit = format!("{path}/visit");
    for (method, uri) in [
        (Method::GET, "/api/v0/subscriptions"),
        (Method::GET, path.as_str()),
        (Method::PUT, path.as_str()),
        (Method::DELETE, path.as_str()),
        (Method::PUT, visit.as_str()),
        (Method::GET, "/api/v0/feed"),
        (Method::POST, "/api/v0/subscriptions/import"),
        (Method::GET, "/api/v0/subscriptions/export?format=newpipe"),
    ] {
        let (status, json) = api.send(method.clone(), uri, false).await;
        assert_eq!(status, StatusCode::UNAUTHORIZED, "{method} {uri}");
        assert_eq!(json["code"], "unauthorized");
    }
}

#[tokio::test]
#[ignore = "requires Docker or OXISTREAM_TEST_DATABASE_URL"]
async fn api_subscribe_triggers_fetch_and_serves_feed() {
    let db = isolated_test_pool().await;
    let channels = Arc::new(PgChannelRepository::new(db.pool.clone()));
    let (feed, mut requests) = Feed::new(channels.clone());
    let api = Api::new(&db.pool, feed).await;
    let path = format!("/api/v0/subscriptions/{CHANNEL_A}");

    let (status, json) = api.send(Method::GET, &path, true).await;
    assert_eq!(
        (status, json["subscribed"].as_bool()),
        (StatusCode::OK, Some(false))
    );

    let (status, json) = api.send(Method::PUT, &path, true).await;
    assert_eq!(status, StatusCode::CREATED, "{json}");
    assert_eq!(json["id"], CHANNEL_A);
    assert!(json["name"].is_null());
    assert_eq!(requests.try_recv().as_deref(), Some(CHANNEL_A));

    let (status, _) = api.send(Method::PUT, &path, true).await;
    assert_eq!(status, StatusCode::OK);
    assert_eq!(
        requests.try_recv(),
        None,
        "only new subscriptions trigger a fetch"
    );

    let (_, json) = api.send(Method::GET, &path, true).await;
    assert_eq!(json["subscribed"], true);

    let videos: Vec<_> = (0..3)
        .map(|i| {
            video(
                &format!("aaaaaaaaaa{i}"),
                datetime!(2026-01-01 00:00 UTC) + time::Duration::days(i),
            )
        })
        .collect();
    channels
        .store_fetch(&channel(CHANNEL_A, "Alpha"), &videos)
        .await
        .unwrap();

    let (_, list) = api.send(Method::GET, "/api/v0/subscriptions", true).await;
    assert_eq!(list[0]["name"], "Alpha");
    assert_eq!(list[0]["has_new"], false, "first fetch");
    sqlx::query("UPDATE subscriptions SET last_visited_at = '2026-01-02 12:00Z'")
        .execute(&db.pool)
        .await
        .unwrap();
    let (_, list) = api.send(Method::GET, "/api/v0/subscriptions", true).await;
    assert_eq!(list[0]["has_new"], true);
    let (status, _) = api.send(Method::PUT, &format!("{path}/visit"), true).await;
    assert_eq!(status, StatusCode::NO_CONTENT);
    let (_, list) = api.send(Method::GET, "/api/v0/subscriptions", true).await;
    assert_eq!(list[0]["has_new"], false, "visited");
    assert!(
        list[0]["avatar"][0]["url"]
            .as_str()
            .unwrap()
            .starts_with("/proxy/image/"),
        "{list}"
    );

    let (status, page) = api.send(Method::GET, "/api/v0/feed?limit=2", true).await;
    assert_eq!(status, StatusCode::OK, "{page}");
    assert_eq!(page["items"][0]["id"], "aaaaaaaaaa2");
    assert_eq!(page["items"][0]["channel"]["name"], "Alpha");
    assert_eq!(page["items"][0]["view_count"], 42);
    assert_eq!(page["items"][0]["published_at"], "2026-01-03T00:00:00Z");
    assert_eq!(page["items"].as_array().unwrap().len(), 2);
    let continuation = page["continuation"].as_str().unwrap();

    let (_, next) = api
        .send(
            Method::GET,
            &format!("/api/v0/feed?limit=2&continuation={continuation}"),
            true,
        )
        .await;
    assert_eq!(next["items"].as_array().unwrap().len(), 1);
    assert_eq!(next["items"][0]["id"], "aaaaaaaaaa0");
    assert!(next["continuation"].is_null());

    let (status, _) = api.send(Method::DELETE, &path, true).await;
    assert_eq!(status, StatusCode::NO_CONTENT);
    let (status, _) = api.send(Method::DELETE, &path, true).await;
    assert_eq!(status, StatusCode::NO_CONTENT, "idempotent");
    let (_, page) = api.send(Method::GET, "/api/v0/feed", true).await;
    assert_eq!(page["items"].as_array().unwrap().len(), 0);
}

#[tokio::test]
#[ignore = "requires Docker or OXISTREAM_TEST_DATABASE_URL"]
async fn api_rejects_bad_input() {
    let db = isolated_test_pool().await;
    let channels = Arc::new(PgChannelRepository::new(db.pool.clone()));
    let api = Api::new(&db.pool, Feed::new(channels).0).await;
    for (method, uri) in [
        (Method::PUT, "/api/v0/subscriptions/nope"),
        (Method::GET, "/api/v0/subscriptions/nope"),
        (Method::DELETE, "/api/v0/subscriptions/nope"),
        (Method::PUT, "/api/v0/subscriptions/nope/visit"),
        (Method::GET, "/api/v0/feed?limit=0"),
        (Method::GET, "/api/v0/feed?limit=101"),
        (Method::GET, "/api/v0/feed?continuation=garbage"),
        (Method::GET, "/api/v0/feed?kind=reels"),
    ] {
        let (status, json) = api.send(method.clone(), uri, true).await;
        assert_eq!(status, StatusCode::BAD_REQUEST, "{method} {uri}: {json}");
    }
}

#[tokio::test]
#[ignore = "requires Docker or OXISTREAM_TEST_DATABASE_URL"]
async fn api_imports_each_format_idempotently() {
    let db = isolated_test_pool().await;
    let channels = Arc::new(PgChannelRepository::new(db.pool.clone()));
    let (feed, mut requests) = Feed::new(channels.clone());
    let api = Api::new(&db.pool, feed).await;
    // Fetched before, so the file's name must not replace YouTube's.
    channels
        .store_fetch(&channel("UCsXVk37bltHxD1rDPwtNM8Q", "Kurzgesagt"), &[])
        .await
        .unwrap();

    let (status, json) = api.import(fixture("takeout_en.csv")).await;
    assert_eq!(status, StatusCode::OK, "{json}");
    assert_eq!(json["format"], "youtube_takeout");
    let counts = |json: &Value| {
        ["imported", "already_subscribed", "skipped", "invalid"].map(|k| json[k].as_u64().unwrap())
    };
    assert_eq!(counts(&json), [4, 0, 0, 0]);
    assert_eq!(
        requests.try_recv(),
        None,
        "the scheduled worker fetches imports"
    );

    let (_, json) = api.import(fixture("takeout_en.csv")).await;
    assert_eq!(counts(&json), [0, 4, 0, 0]);

    let (_, json) = api.import(fixture("newpipe.json")).await;
    assert_eq!(json["format"], "newpipe");
    assert_eq!(counts(&json), [0, 3, 3, 1]);
    assert_eq!(
        json["skipped_entries"][0],
        "https://www.youtube.com/@veritasium"
    );

    for (name, format) in [
        ("invidious.json", "invidious"),
        ("invidious.opml", "opml"),
        ("piped.json", "piped"),
        ("takeout_de.csv", "youtube_takeout"),
    ] {
        let (status, json) = api.import(fixture(name)).await;
        assert_eq!(status, StatusCode::OK, "{name}: {json}");
        assert_eq!(json["format"], format, "{name}");
        assert_eq!(json["imported"], 0, "{name}");
    }

    let (_, list) = api.send(Method::GET, "/api/v0/subscriptions", true).await;
    let names: Vec<_> = list
        .as_array()
        .unwrap()
        .iter()
        .map(|s| s["name"].as_str().unwrap())
        .collect();
    assert_eq!(
        names,
        [
            "Kurzgesagt",
            "Tom Scott, \"quoted\"",
            "Veritasium",
            "Vsauce"
        ]
    );
    let status = channels.status().await.unwrap();
    assert_eq!(
        (status.channels, status.pending),
        (4, 3),
        "names do not count as fetched"
    );
    let due = channels
        .due(OffsetDateTime::now_utc() - INTERVAL, 10)
        .await
        .unwrap();
    assert_eq!(due.len(), 3, "imported channels are due right away");
}

#[tokio::test]
#[ignore = "requires Docker or OXISTREAM_TEST_DATABASE_URL"]
async fn api_rejects_bad_imports() {
    let db = isolated_test_pool().await;
    let channels = Arc::new(PgChannelRepository::new(db.pool.clone()));
    let api = Api::new(&db.pool, Feed::new(channels).0).await;
    for body in [&b""[..], b"hello", b"{\"foo\":1}", b"<rss/>", b"\xff\xfe"] {
        let (status, json) = api.import(body.to_vec()).await;
        assert_eq!(status, StatusCode::BAD_REQUEST, "{json}");
        assert_eq!(json["code"], "bad_request");
    }
    let (status, json) = api.import(vec![b' '; 5 * 1024 * 1024 + 1]).await;
    assert_eq!(status, StatusCode::PAYLOAD_TOO_LARGE, "{json}");

    for uri in [
        "/api/v0/subscriptions/export?format=opml",
        "/api/v0/subscriptions/export",
    ] {
        let (status, json) = api.send(Method::GET, uri, true).await;
        assert_eq!(status, StatusCode::BAD_REQUEST, "{uri}: {json}");
    }
}

#[tokio::test]
#[ignore = "requires Docker or OXISTREAM_TEST_DATABASE_URL"]
async fn api_exports_newpipe_and_invidious() {
    let db = isolated_test_pool().await;
    let channels = Arc::new(PgChannelRepository::new(db.pool.clone()));
    let api = Api::new(&db.pool, Feed::new(channels.clone()).0).await;
    for id in [CHANNEL_B, CHANNEL_A] {
        let (status, _) = api
            .send(Method::PUT, &format!("/api/v0/subscriptions/{id}"), true)
            .await;
        assert_eq!(status, StatusCode::CREATED);
    }
    channels
        .store_fetch(&channel(CHANNEL_A, "Alpha"), &[])
        .await
        .unwrap();

    let (status, headers, body) = api
        .request(
            Method::GET,
            "/api/v0/subscriptions/export?format=newpipe",
            true,
            Body::empty(),
        )
        .await;
    assert_eq!(status, StatusCode::OK);
    assert_eq!(headers[header::CONTENT_TYPE], "application/json");
    let disposition = headers[header::CONTENT_DISPOSITION].to_str().unwrap();
    assert!(
        disposition.starts_with("attachment; filename=\"newpipe_subscriptions_")
            && disposition.ends_with(".json\""),
        "{disposition}"
    );
    let json: Value = serde_json::from_slice(&body).unwrap();
    assert!(json["app_version"].is_string());
    assert_eq!(
        json["subscriptions"],
        serde_json::json!([
            {
                "service_id": 0,
                "url": format!("https://www.youtube.com/channel/{CHANNEL_A}"),
                "name": "Alpha",
            },
            {
                "service_id": 0,
                "url": format!("https://www.youtube.com/channel/{CHANNEL_B}"),
                "name": CHANNEL_B,
            },
        ])
    );

    let (status, _, body) = api
        .request(
            Method::GET,
            "/api/v0/subscriptions/export?format=invidious",
            true,
            Body::empty(),
        )
        .await;
    assert_eq!(status, StatusCode::OK);
    let json: Value = serde_json::from_slice(&body).unwrap();
    assert_eq!(
        json["subscriptions"],
        serde_json::json!([CHANNEL_A, CHANNEL_B])
    );

    let (_, json) = api.import(body).await;
    assert_eq!(
        (
            json["imported"].as_u64(),
            json["already_subscribed"].as_u64()
        ),
        (Some(0), Some(2)),
        "an export imports back unchanged"
    );
}
