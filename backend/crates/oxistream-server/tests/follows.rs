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
    AdvisoryLock, Image, LockKey, NewPlaylist, NewSession, NewUser, PgChannelRepository,
    PgPlaylistFollowRepository, PgPlaylistRepository, PgPool, PgSessionRepository,
    PgUserRepository, PgVideoRepository, PgYouTubePlaylistRepository, PlaylistFollowRepository,
    PlaylistRepository, SessionRepository, User, UserRepository, VideoRepository, VideoUpsert,
    Visibility, YouTubePlaylistItemUpsert, YouTubePlaylistRepository, YouTubePlaylistUpdate,
};
use oxistream_server::AppState;
use oxistream_server::auth::cookie;
use oxistream_server::feed::{
    ChannelFetch, ChannelSource, Feed, FeedWorker, PlaylistFetch, PlaylistSource,
};
use oxistream_server::youtube::{self, YouTubeError};
use serde_json::{Value, json};
use time::OffsetDateTime;
use tower::ServiceExt;

const PLAYLIST_A: &str = "PLaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa";
const PLAYLIST_B: &str = "PLbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbb";
const PLAYLIST_C: &str = "PLcccccccccccccccccccccccccccccccc";
const INTERVAL: Duration = Duration::from_mins(30);

/// The worker lock is per database, not per schema, so worker tests take turns.
static WORKER_LOCK: tokio::sync::Mutex<()> = tokio::sync::Mutex::const_new(());

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

fn playlist(id: &str, title: &str) -> YouTubePlaylistUpdate {
    YouTubePlaylistUpdate {
        id: id.to_owned(),
        title: title.to_owned(),
        channel_id: Some("UCaaaaaaaaaaaaaaaaaaaaaa".into()),
        channel_name: Some("Alpha".into()),
        thumbnails: vec![Image {
            url: "https://i.ytimg.com/vi/x/hqdefault.jpg".into(),
            width: 480,
            height: 360,
        }],
        video_count: Some(3),
    }
}

fn item(id: &str) -> YouTubePlaylistItemUpsert {
    YouTubePlaylistItemUpsert {
        video_id: id.to_owned(),
        title: format!("Video {id}"),
        channel_id: Some("UCaaaaaaaaaaaaaaaaaaaaaa".into()),
        channel_name: Some("Alpha".into()),
        duration_seconds: Some(60),
    }
}

fn items(ids: &[&str]) -> Vec<YouTubePlaylistItemUpsert> {
    ids.iter().map(|id| item(id)).collect()
}

async fn stored(repo: &PgYouTubePlaylistRepository, id: &str) -> Vec<String> {
    repo.items(id)
        .await
        .unwrap()
        .into_iter()
        .map(|i| i.video_id)
        .collect()
}

#[tokio::test]
#[ignore = "requires Docker or OXISTREAM_TEST_DATABASE_URL"]
async fn follow_list_and_unfollow() {
    let db = isolated_test_pool().await;
    let follows = PgPlaylistFollowRepository::new(db.pool.clone());
    let playlists = PgYouTubePlaylistRepository::new(db.pool.clone());
    let alice = user(&db.pool).await;

    let (follow, created) = follows.follow(alice.id, PLAYLIST_B).await.unwrap();
    assert!(created);
    assert_eq!(follow.playlist.title, None, "unknown until fetched");
    let (_, created) = follows.follow(alice.id, PLAYLIST_B).await.unwrap();
    assert!(!created, "following is idempotent");
    follows.follow(alice.id, PLAYLIST_A).await.unwrap();
    playlists
        .store_fetch(&playlist(PLAYLIST_B, "Bravo"), &[])
        .await
        .unwrap();
    playlists
        .store_fetch(&playlist(PLAYLIST_A, "alpha"), &[])
        .await
        .unwrap();

    let list = follows.list(alice.id).await.unwrap();
    let titles: Vec<_> = list.iter().map(|f| f.playlist.title.as_deref()).collect();
    assert_eq!(titles, [Some("alpha"), Some("Bravo")]);
    assert_eq!(list[0].playlist.video_count, Some(3));
    assert!(list[0].playlist.fetched_at.is_some());

    assert!(follows.is_following(alice.id, PLAYLIST_A).await.unwrap());
    assert!(follows.unfollow(alice.id, PLAYLIST_A).await.unwrap());
    assert!(!follows.unfollow(alice.id, PLAYLIST_A).await.unwrap());
    assert!(!follows.is_following(alice.id, PLAYLIST_A).await.unwrap());
    assert!(!follows.mark_seen(alice.id, PLAYLIST_A).await.unwrap());
    assert_eq!(follows.list(alice.id).await.unwrap().len(), 1);
}

#[tokio::test]
#[ignore = "requires Docker or OXISTREAM_TEST_DATABASE_URL"]
async fn fetches_replace_items_and_count_new_ones() {
    let db = isolated_test_pool().await;
    let follows = PgPlaylistFollowRepository::new(db.pool.clone());
    let playlists = PgYouTubePlaylistRepository::new(db.pool.clone());
    let alice = user(&db.pool).await;
    follows.follow(alice.id, PLAYLIST_A).await.unwrap();
    let new_count = async || follows.list(alice.id).await.unwrap()[0].new_count;

    playlists
        .store_fetch(
            &playlist(PLAYLIST_A, "A"),
            &items(&["vvvvvvvvvv0", "vvvvvvvvvv1", "vvvvvvvvvv0", "vvvvvvvvvv4"]),
        )
        .await
        .unwrap();
    let positions: Vec<_> = playlists
        .items(PLAYLIST_A)
        .await
        .unwrap()
        .into_iter()
        .map(|i| (i.position, i.video_id))
        .collect();
    assert_eq!(
        positions,
        [
            (0, "vvvvvvvvvv0".to_owned()),
            (1, "vvvvvvvvvv1".to_owned()),
            (2, "vvvvvvvvvv4".to_owned())
        ],
        "duplicates keep their first place, positions stay dense"
    );
    assert_eq!(new_count().await, 0, "the first fetch is the baseline");

    playlists
        .store_fetch(
            &playlist(PLAYLIST_A, "A2"),
            &items(&["vvvvvvvvvv2", "vvvvvvvvvv1", "vvvvvvvvvv3"]),
        )
        .await
        .unwrap();
    assert_eq!(
        stored(&playlists, PLAYLIST_A).await,
        ["vvvvvvvvvv2", "vvvvvvvvvv1", "vvvvvvvvvv3"]
    );
    let stored_items = playlists.items(PLAYLIST_A).await.unwrap();
    assert!(
        stored_items[1].first_seen_at.is_none(),
        "kept from the baseline"
    );
    assert!(stored_items[0].first_seen_at.is_some());
    assert_eq!(new_count().await, 2);
    let list = follows.list(alice.id).await.unwrap();
    assert_eq!(list[0].playlist.title.as_deref(), Some("A2"));

    assert!(follows.mark_seen(alice.id, PLAYLIST_A).await.unwrap());
    assert_eq!(new_count().await, 0);

    let bob = user(&db.pool).await;
    let (follow, _) = follows.follow(bob.id, PLAYLIST_A).await.unwrap();
    assert_eq!(
        follow.new_count, 0,
        "a late follower starts with nothing new"
    );

    playlists
        .record_failure(PLAYLIST_A, "private")
        .await
        .unwrap();
    let failed = playlists.get(PLAYLIST_A).await.unwrap().unwrap();
    assert_eq!(failed.last_error.as_deref(), Some("private"));
    assert_eq!(
        stored(&playlists, PLAYLIST_A).await.len(),
        3,
        "kept on failure"
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
        Err(YouTubeError::NotFound(channel_id.to_owned()))
    }
}

#[async_trait]
impl PlaylistSource for FakeSource {
    async fn fetch_playlist(&self, playlist_id: &str) -> youtube::Result<PlaylistFetch> {
        *self
            .fetches
            .lock()
            .unwrap()
            .entry(playlist_id.to_owned())
            .or_default() += 1;
        if let Some(err) = self.failures.get(playlist_id) {
            return Err(err.clone());
        }
        let prefix = &playlist_id[2..12];
        Ok(PlaylistFetch {
            // The worker keys rows by the requested ID, not the returned one.
            playlist: playlist("PLcanonical", &format!("Playlist {prefix}")),
            items: vec![item(&format!("{}v", &prefix[..10]))],
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
    .with_playlists(
        Arc::new(PgYouTubePlaylistRepository::new(pool.clone())),
        source.clone(),
    )
}

#[tokio::test]
#[ignore = "requires Docker or OXISTREAM_TEST_DATABASE_URL"]
async fn worker_fetches_each_followed_playlist_once() {
    let _turn = WORKER_LOCK.lock().await;
    let db = isolated_test_pool().await;
    let follows = PgPlaylistFollowRepository::new(db.pool.clone());
    let playlists = PgYouTubePlaylistRepository::new(db.pool.clone());
    let (alice, bob) = (user(&db.pool).await, user(&db.pool).await);
    follows.follow(alice.id, PLAYLIST_A).await.unwrap();
    follows.follow(bob.id, PLAYLIST_A).await.unwrap();
    follows.follow(bob.id, PLAYLIST_B).await.unwrap();
    follows.follow(bob.id, PLAYLIST_C).await.unwrap();
    let source = Arc::new(FakeSource::failing(&[(
        PLAYLIST_C,
        YouTubeError::NotFound("gone".into()),
    )]));
    let worker = worker(&db.pool, &source);

    let summary = worker.run_once().await.unwrap().expect("lock is free");
    assert_eq!(
        (summary.playlists_fetched, summary.playlists_failed),
        (2, 1)
    );
    assert_eq!(source.fetches(PLAYLIST_A), 1, "once despite two followers");
    let fetched = playlists.get(PLAYLIST_A).await.unwrap().unwrap();
    assert_eq!(fetched.title.as_deref(), Some("Playlist aaaaaaaaaa"));
    assert_eq!(stored(&playlists, PLAYLIST_A).await, ["aaaaaaaaaav"]);
    let failed = playlists.get(PLAYLIST_C).await.unwrap().unwrap();
    assert!(failed.last_error.unwrap().contains("gone"));
    assert!(failed.fetched_at.is_none());

    let again = worker.run_once().await.unwrap().unwrap();
    assert_eq!(again.playlists_fetched + again.playlists_failed, 0);
    assert_eq!(source.total(), 3);

    // Nobody follows A anymore: its items go, and a new follow starts from scratch.
    follows.unfollow(alice.id, PLAYLIST_A).await.unwrap();
    follows.unfollow(bob.id, PLAYLIST_A).await.unwrap();
    let pruned = worker.run_once().await.unwrap().unwrap();
    assert_eq!(pruned.playlists_pruned, 1);
    assert_eq!(stored(&playlists, PLAYLIST_A).await, Vec::<String>::new());
    let reset = playlists.get(PLAYLIST_A).await.unwrap().unwrap();
    assert!(reset.fetched_at.is_none() && reset.checked_at.is_none());
}

#[tokio::test]
#[ignore = "requires Docker or OXISTREAM_TEST_DATABASE_URL"]
async fn worker_stops_playlists_when_rate_limited() {
    let _turn = WORKER_LOCK.lock().await;
    let db = isolated_test_pool().await;
    let follows = PgPlaylistFollowRepository::new(db.pool.clone());
    let alice = user(&db.pool).await;
    follows.follow(alice.id, PLAYLIST_A).await.unwrap();
    follows.follow(alice.id, PLAYLIST_B).await.unwrap();
    let source = Arc::new(FakeSource::failing(&[(
        PLAYLIST_A,
        YouTubeError::RateLimited("captcha".into()),
    )]));

    let summary = worker(&db.pool, &source).run_once().await.unwrap().unwrap();
    assert!(summary.rate_limited);
    assert_eq!(source.total(), 1, "stops at the first block");
}

#[tokio::test]
#[ignore = "requires Docker or OXISTREAM_TEST_DATABASE_URL"]
async fn worker_skips_while_another_replica_holds_the_lock() {
    let _turn = WORKER_LOCK.lock().await;
    let db = isolated_test_pool().await;
    let follows = PgPlaylistFollowRepository::new(db.pool.clone());
    let alice = user(&db.pool).await;
    follows.follow(alice.id, PLAYLIST_A).await.unwrap();
    let source = Arc::new(FakeSource::default());

    let held = AdvisoryLock::try_acquire(&db.pool, LockKey::FeedWorker)
        .await
        .unwrap()
        .unwrap();
    assert_eq!(worker(&db.pool, &source).run_once().await.unwrap(), None);
    held.release().await.unwrap();
    assert_eq!(source.total(), 0);
}

#[tokio::test]
#[ignore = "requires Docker or OXISTREAM_TEST_DATABASE_URL"]
async fn requested_fetch_skips_fresh_playlists() {
    let db = isolated_test_pool().await;
    let follows = PgPlaylistFollowRepository::new(db.pool.clone());
    let alice = user(&db.pool).await;
    follows.follow(alice.id, PLAYLIST_A).await.unwrap();
    let source = Arc::new(FakeSource::default());
    let worker = worker(&db.pool, &source);

    assert!(worker.fetch_requested_playlist(PLAYLIST_A).await.unwrap());
    assert!(!worker.fetch_requested_playlist(PLAYLIST_A).await.unwrap());
    assert_eq!(source.fetches(PLAYLIST_A), 1);
}

#[tokio::test]
#[ignore = "requires Docker or OXISTREAM_TEST_DATABASE_URL"]
async fn import_creates_a_playlist_with_items_in_one_go() {
    let db = isolated_test_pool().await;
    let repo = PgPlaylistRepository::new(db.pool.clone());
    let videos = PgVideoRepository::new(db.pool.clone());
    let alice = user(&db.pool).await;
    let video = |id: &str, title: &str| VideoUpsert {
        id: id.to_owned(),
        title: title.to_owned(),
        channel_id: "UCaaaaaaaaaaaaaaaaaaaaaa".into(),
        channel_name: "Alpha".into(),
        duration_seconds: Some(60),
        published_at: None,
        thumbnail_url: None,
    };
    videos
        .upsert(&video("vvvvvvvvvv1", "Cached title"))
        .await
        .unwrap();
    let new = NewPlaylist {
        id: "pppppppppppppppI".into(),
        title: "Imported".into(),
        description: "From YouTube".into(),
        visibility: Visibility::Private,
    };
    let created = repo
        .create_with_items(
            alice.id,
            &new,
            &[
                video("vvvvvvvvvv2", "Two"),
                video("vvvvvvvvvv1", "One"),
                video("vvvvvvvvvv2", "Two again"),
            ],
            10,
        )
        .await
        .unwrap();
    assert_eq!(created.item_count, 2);
    assert_eq!(created.first_video_id.as_deref(), Some("vvvvvvvvvv2"));
    let items = repo.items(&new.id).await.unwrap();
    let order: Vec<_> = items
        .iter()
        .map(|i| (i.position, i.video.title.as_str()))
        .collect();
    assert_eq!(order, [(0, "Two"), (1, "Cached title")], "cache wins");

    let full = repo
        .create_with_items(
            alice.id,
            &NewPlaylist {
                id: "pppppppppppppppJ".into(),
                ..new
            },
            &[],
            1,
        )
        .await;
    assert!(matches!(full, Err(oxistream_db::Error::LimitReached)));
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
}

fn feed(pool: &PgPool) -> (Feed, oxistream_server::feed::FeedRequests) {
    Feed::new(Arc::new(PgChannelRepository::new(pool.clone())))
}

#[tokio::test]
#[ignore = "requires Docker or OXISTREAM_TEST_DATABASE_URL"]
async fn api_requires_login() {
    let db = isolated_test_pool().await;
    let api = Api::new(&db.pool, feed(&db.pool).0).await;
    let path = format!("/api/v0/playlist-follows/{PLAYLIST_A}");
    let seen = format!("{path}/seen");
    for (method, uri) in [
        (Method::GET, "/api/v0/playlist-follows"),
        (Method::GET, path.as_str()),
        (Method::PUT, path.as_str()),
        (Method::DELETE, path.as_str()),
        (Method::PUT, seen.as_str()),
        (Method::POST, "/api/v0/playlists/import"),
    ] {
        let (status, json) = api.send(method.clone(), uri, false, None).await;
        assert_eq!(status, StatusCode::UNAUTHORIZED, "{method} {uri}");
        assert_eq!(json["code"], "unauthorized");
    }
}

#[tokio::test]
#[ignore = "requires Docker or OXISTREAM_TEST_DATABASE_URL"]
async fn api_follow_triggers_fetch_and_lists_playlists() {
    let db = isolated_test_pool().await;
    let (feed, mut requests) = feed(&db.pool);
    let api = Api::new(&db.pool, feed).await;
    let path = format!("/api/v0/playlist-follows/{PLAYLIST_A}");

    let (status, json) = api.send(Method::GET, &path, true, None).await;
    assert_eq!(
        (status, json["following"].as_bool()),
        (StatusCode::OK, Some(false))
    );

    let (status, json) = api.send(Method::PUT, &path, true, None).await;
    assert_eq!(status, StatusCode::CREATED, "{json}");
    assert_eq!(json["id"], PLAYLIST_A);
    assert!(json["title"].is_null());
    assert_eq!(json["new_video_count"], 0);
    assert_eq!(requests.try_recv_playlist().as_deref(), Some(PLAYLIST_A));
    assert_eq!(requests.try_recv(), None, "no channel fetch");

    let (status, _) = api.send(Method::PUT, &path, true, None).await;
    assert_eq!(status, StatusCode::OK);
    assert_eq!(
        requests.try_recv_playlist(),
        None,
        "only new follows trigger a fetch"
    );

    PgYouTubePlaylistRepository::new(db.pool.clone())
        .store_fetch(&playlist(PLAYLIST_A, "Alpha mix"), &items(&["vvvvvvvvvv0"]))
        .await
        .unwrap();
    let (status, json) = api
        .send(Method::GET, "/api/v0/playlist-follows", true, None)
        .await;
    assert_eq!(status, StatusCode::OK);
    let list = json.as_array().unwrap();
    assert_eq!(list.len(), 1);
    assert_eq!(list[0]["title"], "Alpha mix");
    assert_eq!(list[0]["channel"]["name"], "Alpha");
    assert_eq!(list[0]["video_count"], 3);
    assert_eq!(list[0]["unavailable"], false);
    let thumb = list[0]["thumbnails"][0]["url"].as_str().unwrap();
    assert!(thumb.starts_with("/proxy/image/"), "{thumb}");

    let (status, _) = api
        .send(Method::PUT, &format!("{path}/seen"), true, None)
        .await;
    assert_eq!(status, StatusCode::NO_CONTENT);

    let (status, _) = api.send(Method::DELETE, &path, true, None).await;
    assert_eq!(status, StatusCode::NO_CONTENT);
    let (status, _) = api.send(Method::DELETE, &path, true, None).await;
    assert_eq!(status, StatusCode::NO_CONTENT, "idempotent");
    let (status, json) = api
        .send(Method::PUT, &format!("{path}/seen"), true, None)
        .await;
    assert_eq!(status, StatusCode::NOT_FOUND, "{json}");
}

#[tokio::test]
#[ignore = "requires Docker or OXISTREAM_TEST_DATABASE_URL"]
async fn api_rejects_mixes_and_bad_ids() {
    let db = isolated_test_pool().await;
    let api = Api::new(&db.pool, feed(&db.pool).0).await;

    for method in [Method::GET, Method::PUT, Method::DELETE] {
        let (status, json) = api
            .send(
                method.clone(),
                "/api/v0/playlist-follows/RDdQw4w9WgXcQ",
                true,
                None,
            )
            .await;
        assert_eq!(status, StatusCode::NOT_FOUND, "{method}");
        assert_eq!(json["reason"], "mix");
        let (status, _) = api
            .send(method.clone(), "/api/v0/playlist-follows/WL", true, None)
            .await;
        assert_eq!(status, StatusCode::BAD_REQUEST, "{method}");
    }

    let (status, json) = api
        .send(
            Method::POST,
            "/api/v0/playlists/import",
            true,
            Some(json!({ "playlist_id": "RDdQw4w9WgXcQ" })),
        )
        .await;
    assert_eq!(
        (status, json["reason"].as_str()),
        (StatusCode::NOT_FOUND, Some("mix"))
    );
    let (status, _) = api
        .send(
            Method::POST,
            "/api/v0/playlists/import",
            true,
            Some(json!({ "playlist_id": "not a playlist" })),
        )
        .await;
    assert_eq!(status, StatusCode::BAD_REQUEST);
    let (status, _) = api
        .send(
            Method::POST,
            "/api/v0/playlists/import",
            true,
            Some(json!({})),
        )
        .await;
    assert!(status.is_client_error(), "{status}");
}
