#![allow(dead_code)]

pub mod idp;

use std::time::Duration;

use axum::Router;
use axum::body::Body;
use axum::http::{Request, StatusCode};
use http_body_util::BodyExt;
use std::sync::Arc;

use oxistream_db::{
    PgChannelBlockRepository, PgChannelRepository, PgHistoryRepository, PgLikeRepository,
    PgPlaylistFollowRepository, PgPlaylistRepository, PgPool, PgQueueRepository, PgRoomRepository,
    PgSessionRepository, PgSponsorBlockSettingsRepository, PgSubscriptionRepository,
    PgUserRepository, PgUserSettingsRepository, PgVideoRepository, PoolOptions,
};
use oxistream_server::auth::Auth;
use oxistream_server::config::{AuthConfig, ProxyConfig, SponsorBlockConfig, YouTubeConfig};
use oxistream_server::feed::Feed;
use oxistream_server::proxy::Proxy;
use oxistream_server::rooms::Rooms;
use oxistream_server::sponsorblock::SponsorBlock;
use oxistream_server::youtube::YouTube;
use oxistream_server::{AppState, telemetry};
use serde_json::json;
use testcontainers_modules::postgres::Postgres;
use testcontainers_modules::testcontainers::runners::AsyncRunner;
use testcontainers_modules::testcontainers::{ContainerAsync, ImageExt};
use tower::ServiceExt;

pub const TEST_DATABASE_URL_ENV: &str = "OXISTREAM_TEST_DATABASE_URL";
pub const TEST_PROXY_SECRET: &str = "test-secret-test-secret-test-secret";

/// A migrated database pool. Holds the container, if one was started, so it outlives the test.
pub struct TestDb {
    pub pool: PgPool,
    _container: Option<ContainerAsync<Postgres>>,
}

/// Uses `OXISTREAM_TEST_DATABASE_URL` when set, else starts a Postgres 18 container.
pub async fn test_pool() -> TestDb {
    let (url, container) = database_url().await;
    migrated(&url, container).await
}

/// Like [`test_pool`], but in a fresh schema, for tests that need empty tables.
pub async fn isolated_test_pool() -> TestDb {
    let (url, container) = database_url().await;
    let schema = format!("test_{}", unique_suffix());
    let admin = oxistream_db::connect(&url, &PoolOptions::default())
        .await
        .expect("connect to test database");
    sqlx::query(sqlx::AssertSqlSafe(format!("CREATE SCHEMA {schema}")))
        .execute(&admin)
        .await
        .expect("create test schema");
    admin.close().await;
    let separator = if url.contains('?') { '&' } else { '?' };
    let url = format!("{url}{separator}options=-c%20search_path%3D{schema}");
    migrated(&url, container).await
}

/// Unique per call and per test run, for usernames and schema names.
pub fn unique_suffix() -> String {
    static COUNTER: std::sync::atomic::AtomicU32 = std::sync::atomic::AtomicU32::new(0);
    let nanos = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap()
        .as_nanos();
    let count = COUNTER.fetch_add(1, std::sync::atomic::Ordering::Relaxed);
    format!("{:x}{count:x}", nanos & 0xffff_ffff_ffff)
}

async fn database_url() -> (String, Option<ContainerAsync<Postgres>>) {
    if let Ok(url) = std::env::var(TEST_DATABASE_URL_ENV) {
        (url, None)
    } else {
        let container = Postgres::default()
            .with_tag("18-alpine")
            .start()
            .await
            .expect("start postgres container");
        let host = container.get_host().await.unwrap();
        let port = container.get_host_port_ipv4(5432).await.unwrap();
        let url = format!("postgres://postgres:postgres@{host}:{port}/postgres");
        (url, Some(container))
    }
}

async fn migrated(url: &str, container: Option<ContainerAsync<Postgres>>) -> TestDb {
    let pool = oxistream_db::connect(url, &PoolOptions::default())
        .await
        .expect("connect to test database");
    oxistream_db::migrate(&pool)
        .await
        .expect("migrate test database");
    TestDb {
        pool,
        _container: container,
    }
}

pub fn state_with_pool(db: PgPool) -> AppState {
    state_with_auth(db, AuthConfig::default())
}

pub fn state_with_auth(db: PgPool, auth: AuthConfig) -> AppState {
    AppState {
        auth: Auth::new(
            auth,
            Arc::new(PgUserRepository::new(db.clone())),
            Arc::new(PgSessionRepository::new(db.clone())),
            None,
        ),
        videos: Arc::new(PgVideoRepository::new(db.clone())),
        subscriptions: Arc::new(PgSubscriptionRepository::new(db.clone())),
        history: Arc::new(PgHistoryRepository::new(db.clone())),
        settings: Arc::new(PgUserSettingsRepository::new(db.clone())),
        playlists: Arc::new(PgPlaylistRepository::new(db.clone())),
        follows: Arc::new(PgPlaylistFollowRepository::new(db.clone())),
        likes: Arc::new(PgLikeRepository::new(db.clone())),
        queue: Arc::new(PgQueueRepository::new(db.clone())),
        blocks: Arc::new(PgChannelBlockRepository::new(db.clone())),
        // Fetch requests go nowhere unless a test swaps in its own feed.
        feed: Feed::new(Arc::new(PgChannelRepository::new(db.clone()))).0,
        rooms: Rooms::new(Arc::new(PgRoomRepository::new(db.clone()))),
        metrics: telemetry::detached_metrics_handle(),
        youtube: YouTube::new(&YouTubeConfig::default()).expect("build YouTube client"),
        proxy: Proxy::new(&ProxyConfig::new(TEST_PROXY_SECRET)).expect("build proxy"),
        rate_limit: None,
        // Unreachable, so no test talks to the real SponsorBlock by accident.
        sponsorblock: SponsorBlock::new(&SponsorBlockConfig {
            enabled: true,
            api_url: "http://127.0.0.1:1".into(),
        })
        .expect("build SponsorBlock client"),
        sponsorblock_settings: Arc::new(PgSponsorBlockSettingsRepository::new(db.clone())),
        db,
    }
}

pub fn app_with_pool(db: PgPool) -> Router {
    oxistream_server::app(state_with_pool(db))
}

/// A pool pointing at a closed port, for routes that must not need the database.
pub fn unreachable_pool() -> PgPool {
    let options = PoolOptions {
        max_connections: 1,
        acquire_timeout: Duration::from_millis(500),
    };
    oxistream_db::connect_lazy("postgres://oxistream@127.0.0.1:1/oxistream", &options)
        .expect("valid database url")
}

/// State without a reachable database, for routes that must not need it.
pub fn state_without_db() -> AppState {
    state_with_pool(unreachable_pool())
}

pub fn app_without_db() -> Router {
    oxistream_server::app(state_without_db())
}

pub async fn get(app: Router, uri: &str) -> (StatusCode, String) {
    let response = app
        .oneshot(Request::get(uri).body(Body::empty()).unwrap())
        .await
        .unwrap();
    let status = response.status();
    let body = response.into_body().collect().await.unwrap().to_bytes();
    (status, String::from_utf8(body.to_vec()).unwrap())
}

/// Serves `router` on an ephemeral local port and returns its base URL.
pub async fn spawn_upstream(router: Router) -> reqwest::Url {
    let listener = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
    let addr = listener.local_addr().unwrap();
    tokio::spawn(async move { axum::serve(listener, router).await.unwrap() });
    format!("http://{addr}/").parse().unwrap()
}

/// Two manual English tracks and an auto-generated one.
fn fake_subtitles(base: &str, video_id: &str) -> serde_json::Value {
    json!([
        {
            "url": format!("{base}api/timedtext?v={video_id}&lang=en&fmt=srv3"),
            "lang": "en",
            "lang_name": "English",
            "auto_generated": false,
        },
        {
            "url": format!("{base}api/timedtext?v={video_id}&lang=en&kind=asr"),
            "lang": "en",
            "lang_name": "English (auto-generated)",
            "auto_generated": true,
        },
        {
            "url": format!("{base}api/timedtext?v={video_id}&lang=en&name=English"),
            "lang": "en",
            "lang_name": "English - English",
            "auto_generated": false,
        },
    ])
}

/// Player data whose stream and subtitle URLs point below `base`.
pub fn fake_player(base: &str, video_id: &str) -> rustypipe::model::VideoPlayer {
    let video = |itag: u32, mime: &str, width: u32, height: u32, hdr: bool| {
        json!({
            "url": format!("{base}videoplayback?itag={itag}"),
            "itag": itag,
            "bitrate": 1_000_000,
            "average_bitrate": 900_000,
            "size": 1_000_000,
            "index_range": { "start": 741, "end": 1204 },
            "init_range": { "start": 0, "end": 740 },
            "duration_ms": 213_000,
            "width": width,
            "height": height,
            "fps": 30,
            "quality": format!("{height}p"),
            "hdr": hdr,
            "mime": mime,
            "format": if mime.starts_with("video/mp4") { "mp4" } else { "webm" },
            "codec": if mime.contains("avc1") { "avc1" } else { "vp9" },
            "drm_track_type": null,
            "drm_systems": [],
        })
    };
    let audio = |itag: u32, id: &str, lang_name: &str, is_default: bool| {
        json!({
            "url": format!("{base}videoplayback?itag={itag}&track={id}"),
            "itag": itag,
            "bitrate": 130_000,
            "average_bitrate": 128_000,
            "size": 3_000_000,
            "index_range": { "start": 266, "end": 621 },
            "init_range": { "start": 0, "end": 265 },
            "duration_ms": 213_000,
            "mime": "audio/webm; codecs=\"opus\"",
            "format": "webm",
            "codec": "opus",
            "channels": 2,
            "loudness_db": null,
            "track": {
                "id": id,
                "lang": id.split('.').next(),
                "lang_name": lang_name,
                "is_default": is_default,
                "track_type": if is_default { "original" } else { "dubbed" },
            },
            "drm_track_type": null,
            "drm_systems": [],
        })
    };
    serde_json::from_value(json!({
        "details": {
            "id": video_id,
            "name": "Fixture",
            "description": null,
            "duration": 213,
            "thumbnail": [],
            "channel_id": "UCuAXFkgsw1L7xaCfnd5JJOw",
            "channel_name": "Fixture",
            "view_count": 1,
            "keywords": [],
            "is_live": false,
            "is_live_content": false,
        },
        "video_streams": [],
        "video_only_streams": [
            video(137, "video/mp4; codecs=\"avc1.640028\"", 1920, 1080, false),
            video(337, "video/webm; codecs=\"vp09.02.51.10.01.09.16.09.00\"", 3840, 2160, true),
        ],
        "audio_streams": [
            audio(251, "en.4", "English original", true),
            audio(251, "de.3", "German", false),
        ],
        "subtitles": fake_subtitles(base, video_id),
        "expires_in_seconds": 21_540,
        "valid_until": "2099-01-01T00:00:00Z",
        "hls_manifest_url": null,
        "dash_manifest_url": null,
        "preview_frames": [],
        "drm": null,
        "client_type": "desktop",
        "visitor_data": null,
    }))
    .expect("valid player fixture")
}
