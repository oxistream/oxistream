//! Needs `OXISTREAM_TEST_DATABASE_URL` or a Docker daemon; run with `cargo test -- --include-ignored`.

mod common;

use axum::Router;
use axum::body::Body;
use axum::http::{Method, Request, StatusCode, header};
use common::{isolated_test_pool, state_with_pool, unique_suffix};
use http_body_util::BodyExt;
use oxistream_db::Uuid;
use oxistream_db::{
    ChannelRepository, ChannelUpdate, ChannelVideoUpsert, HistoryRepository, NewSession, NewUser,
    PgChannelRepository, PgHistoryRepository, PgPool, PgSessionRepository,
    PgSubscriptionRepository, PgUserRepository, PgUserSettingsRepository, PgVideoRepository,
    ProgressUpdate, SessionRepository, SubscriptionRepository, User, UserRepository,
    UserSettingsRepository, VideoRepository, VideoUpsert,
};
use oxistream_server::AppState;
use oxistream_server::auth::cookie;
use oxistream_server::retention::{self, Retention};
use serde_json::{Value, json};
use std::sync::Arc;
use time::OffsetDateTime;
use tower::ServiceExt;

const VIDEO_A: &str = "aaaaaaaaaaa";
const VIDEO_B: &str = "bbbbbbbbbbb";
const VIDEO_C: &str = "ccccccccccc";
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

fn update(video_id: &str, position: f64, finished: bool) -> ProgressUpdate {
    ProgressUpdate {
        video_id: video_id.to_owned(),
        position_seconds: position,
        duration_seconds: 200.0,
        finished,
    }
}

async fn cache_video(pool: &PgPool, id: &str) {
    PgVideoRepository::new(pool.clone())
        .upsert(&VideoUpsert {
            id: id.to_owned(),
            title: format!("Video {id}"),
            channel_id: CHANNEL.into(),
            channel_name: "Alpha".into(),
            duration_seconds: Some(200),
            published_at: None,
            thumbnail_url: None,
        })
        .await
        .unwrap();
}

#[tokio::test]
#[ignore = "requires Docker or OXISTREAM_TEST_DATABASE_URL"]
async fn record_list_and_delete() {
    let db = isolated_test_pool().await;
    let history = PgHistoryRepository::new(db.pool.clone());
    let alice = user(&db.pool).await;
    let bob = user(&db.pool).await;
    cache_video(&db.pool, VIDEO_A).await;

    assert!(
        history
            .record(alice.id, &update(VIDEO_A, 30.0, false))
            .await
            .unwrap()
    );
    assert!(
        history
            .record(alice.id, &update(VIDEO_B, 5.0, false))
            .await
            .unwrap()
    );
    assert!(
        history
            .record(alice.id, &update(VIDEO_C, 190.0, true))
            .await
            .unwrap()
    );
    assert!(
        history
            .record(bob.id, &update(VIDEO_A, 50.0, false))
            .await
            .unwrap()
    );
    // Watching A again moves it to the top and replaces its progress.
    assert!(
        history
            .record(alice.id, &update(VIDEO_A, 42.5, false))
            .await
            .unwrap()
    );

    let list = history.list(alice.id, None, None, 2).await.unwrap();
    let ids: Vec<_> = list.iter().map(|e| e.progress.video_id.as_str()).collect();
    assert_eq!(ids, [VIDEO_A, VIDEO_C]);
    assert_eq!(list[0].progress.position_seconds, 42.5);
    assert_eq!(list[0].title.as_deref(), Some("Video aaaaaaaaaaa"));
    assert_eq!(list[0].channel_name.as_deref(), Some("Alpha"));
    assert_eq!(list[1].title, None, "metadata not cached");

    let rest = history
        .list(alice.id, None, Some(&list[1].cursor()), 2)
        .await
        .unwrap();
    let ids: Vec<_> = rest.iter().map(|e| e.progress.video_id.as_str()).collect();
    assert_eq!(ids, [VIDEO_B]);

    let unfinished = history.unfinished(alice.id, 10.0, 10).await.unwrap();
    let ids: Vec<_> = unfinished
        .iter()
        .map(|e| e.progress.video_id.as_str())
        .collect();
    assert_eq!(ids, [VIDEO_A], "finished and barely started are left out");

    let progress = history
        .progress(
            alice.id,
            &[VIDEO_A.into(), VIDEO_C.into(), "zzzzzzzzzzz".into()],
        )
        .await
        .unwrap();
    assert_eq!(progress.len(), 2);
    assert_eq!(history.progress(alice.id, &[]).await.unwrap(), []);

    assert!(history.delete(alice.id, VIDEO_A).await.unwrap());
    assert!(!history.delete(alice.id, VIDEO_A).await.unwrap());
    assert_eq!(history.clear(alice.id).await.unwrap(), 2);
    assert_eq!(history.list(alice.id, None, None, 10).await.unwrap(), []);
    assert_eq!(history.list(bob.id, None, None, 10).await.unwrap().len(), 1);
}

#[tokio::test]
#[ignore = "requires Docker or OXISTREAM_TEST_DATABASE_URL"]
async fn paused_history_records_nothing() {
    let db = isolated_test_pool().await;
    let history = PgHistoryRepository::new(db.pool.clone());
    let settings = PgUserSettingsRepository::new(db.pool.clone());
    let alice = user(&db.pool).await;

    assert!(!settings.get(alice.id).await.unwrap().history_paused);
    assert!(
        history
            .record(alice.id, &update(VIDEO_A, 30.0, false))
            .await
            .unwrap()
    );
    assert!(
        settings
            .set_history_paused(alice.id, true)
            .await
            .unwrap()
            .history_paused
    );
    assert!(
        !history
            .record(alice.id, &update(VIDEO_A, 60.0, false))
            .await
            .unwrap()
    );
    assert!(
        !history
            .record(alice.id, &update(VIDEO_B, 60.0, false))
            .await
            .unwrap()
    );
    let list = history.list(alice.id, None, None, 10).await.unwrap();
    assert_eq!(list.len(), 1, "existing history stays");
    assert_eq!(list[0].progress.position_seconds, 30.0);

    assert!(
        !settings
            .set_history_paused(alice.id, false)
            .await
            .unwrap()
            .history_paused
    );
    assert!(
        history
            .record(alice.id, &update(VIDEO_B, 60.0, false))
            .await
            .unwrap()
    );
}

#[tokio::test]
#[ignore = "requires Docker or OXISTREAM_TEST_DATABASE_URL"]
async fn entries_use_subscribed_channel_metadata() {
    let db = isolated_test_pool().await;
    let history = PgHistoryRepository::new(db.pool.clone());
    let alice = user(&db.pool).await;
    cache_video(&db.pool, VIDEO_A).await;
    PgSubscriptionRepository::new(db.pool.clone())
        .subscribe(alice.id, CHANNEL)
        .await
        .unwrap();
    PgChannelRepository::new(db.pool.clone())
        .store_fetch(
            &ChannelUpdate {
                id: CHANNEL.into(),
                name: "Alpha renamed".into(),
                handle: None,
                avatar: vec![oxistream_db::Image {
                    url: "https://yt3.ggpht.com/a".into(),
                    width: 88,
                    height: 88,
                }],
                subscriber_count: None,
                verified: true,
            },
            &[],
        )
        .await
        .unwrap();
    history
        .record(alice.id, &update(VIDEO_A, 30.0, false))
        .await
        .unwrap();
    let list = history.list(alice.id, None, None, 10).await.unwrap();
    assert_eq!(list[0].channel_name.as_deref(), Some("Alpha renamed"));
    assert_eq!(list[0].channel_avatar.len(), 1);
    assert!(list[0].channel_verified);
}

#[tokio::test]
#[ignore = "requires Docker or OXISTREAM_TEST_DATABASE_URL"]
async fn known_shorts_are_neither_recorded_nor_continued() {
    let db = isolated_test_pool().await;
    let history = PgHistoryRepository::new(db.pool.clone());
    let alice = user(&db.pool).await;
    // B was watched before the feed worker learned it is a short.
    for id in [VIDEO_A, VIDEO_B] {
        assert!(
            history
                .record(alice.id, &update(id, 30.0, false))
                .await
                .unwrap()
        );
    }
    let short = |id: &str| ChannelVideoUpsert {
        video_id: id.into(),
        title: format!("Short {id}"),
        duration_seconds: Some(30),
        published_at: None,
        published_text: None,
        thumbnails: vec![],
        view_count: None,
        is_live: false,
        is_upcoming: false,
        is_short: true,
    };
    PgChannelRepository::new(db.pool.clone())
        .store_fetch(
            &ChannelUpdate {
                id: CHANNEL.into(),
                name: "Alpha".into(),
                handle: None,
                avatar: vec![],
                subscriber_count: None,
                verified: false,
            },
            &[short(VIDEO_B), short(VIDEO_C)],
        )
        .await
        .unwrap();

    assert!(
        !history
            .record(alice.id, &update(VIDEO_C, 30.0, false))
            .await
            .unwrap()
    );
    let unfinished = history.unfinished(alice.id, 10.0, 10).await.unwrap();
    let ids: Vec<_> = unfinished
        .iter()
        .map(|e| e.progress.video_id.as_str())
        .collect();
    assert_eq!(ids, [VIDEO_A]);
    let list = history.list(alice.id, None, None, 10).await.unwrap();
    assert_eq!(list.len(), 2, "earlier history stays");
}

#[tokio::test]
#[ignore = "requires Docker or OXISTREAM_TEST_DATABASE_URL"]
async fn dismissed_videos_leave_continue_watching_until_watched_again() {
    let db = isolated_test_pool().await;
    let history = PgHistoryRepository::new(db.pool.clone());
    let alice = user(&db.pool).await;
    let bob = user(&db.pool).await;
    for id in [VIDEO_A, VIDEO_B] {
        history
            .record(alice.id, &update(id, 30.0, false))
            .await
            .unwrap();
    }
    history
        .record(bob.id, &update(VIDEO_A, 30.0, false))
        .await
        .unwrap();

    assert!(history.dismiss_continue(alice.id, VIDEO_A).await.unwrap());
    assert!(!history.dismiss_continue(alice.id, VIDEO_C).await.unwrap());
    let ids = |entries: Vec<oxistream_db::HistoryEntry>| {
        entries
            .into_iter()
            .map(|e| e.progress.video_id)
            .collect::<Vec<_>>()
    };
    assert_eq!(
        ids(history.unfinished(alice.id, 10.0, 10).await.unwrap()),
        [VIDEO_B]
    );
    assert_eq!(
        history.list(alice.id, None, None, 10).await.unwrap().len(),
        2,
        "history keeps it"
    );
    let resume = history
        .progress(alice.id, &[VIDEO_A.to_owned()])
        .await
        .unwrap();
    assert_eq!(resume[0].position_seconds, 30.0, "resume point stays");
    assert_eq!(
        ids(history.unfinished(bob.id, 10.0, 10).await.unwrap()),
        [VIDEO_A],
        "other users are unaffected"
    );

    history
        .record(alice.id, &update(VIDEO_A, 40.0, false))
        .await
        .unwrap();
    assert_eq!(
        ids(history.unfinished(alice.id, 10.0, 10).await.unwrap()),
        [VIDEO_A, VIDEO_B],
        "watching again brings it back"
    );
}

struct Api {
    app: Router,
    token: String,
    user_id: Uuid,
}

impl Api {
    async fn new(pool: &PgPool) -> Self {
        Self::with_max_retention(pool, Retention::Forever).await
    }

    async fn with_max_retention(pool: &PgPool, max: Retention) -> Self {
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
            history: Arc::new(
                PgHistoryRepository::new(pool.clone()).with_max_retention_days(max.days()),
            ),
            history_max_retention: max,
            ..state_with_pool(pool.clone())
        });
        Self {
            app,
            token,
            user_id: user.id,
        }
    }

    async fn request(
        &self,
        method: Method,
        uri: &str,
        body: Option<(&str, String)>,
        signed_in: bool,
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
            Some((content_type, body)) => {
                req = req.header(header::CONTENT_TYPE, content_type);
                Body::from(body)
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

    async fn send(&self, method: Method, uri: &str) -> (StatusCode, Value) {
        self.request(method, uri, None, true).await
    }

    async fn report(&self, content_type: &str, body: &Value) -> StatusCode {
        self.request(
            Method::POST,
            "/api/v0/progress",
            Some((content_type, body.to_string())),
            true,
        )
        .await
        .0
    }
}

fn report(video_id: &str, position: f64, finished: bool) -> Value {
    json!({
        "video_id": video_id,
        "position_seconds": position,
        "duration_seconds": 200.0,
        "finished": finished,
    })
}

#[tokio::test]
#[ignore = "requires Docker or OXISTREAM_TEST_DATABASE_URL"]
async fn api_requires_login() {
    let db = isolated_test_pool().await;
    let api = Api::new(&db.pool).await;
    let entry = format!("/api/v0/history/{VIDEO_A}");
    let dismiss = format!("/api/v0/history/continue/{VIDEO_A}");
    let one = format!("/api/v0/progress/{VIDEO_A}");
    let batch = format!("/api/v0/progress?ids={VIDEO_A}");
    for (method, uri) in [
        (Method::POST, "/api/v0/progress"),
        (Method::GET, batch.as_str()),
        (Method::GET, one.as_str()),
        (Method::GET, "/api/v0/history"),
        (Method::DELETE, "/api/v0/history"),
        (Method::DELETE, entry.as_str()),
        (Method::GET, "/api/v0/history/continue"),
        (Method::DELETE, dismiss.as_str()),
        (Method::GET, "/api/v0/history/settings"),
        (Method::PUT, "/api/v0/history/settings"),
    ] {
        let (status, json) = api.request(method.clone(), uri, None, false).await;
        assert_eq!(status, StatusCode::UNAUTHORIZED, "{method} {uri}");
        assert_eq!(json["code"], "unauthorized");
    }
}

#[tokio::test]
#[ignore = "requires Docker or OXISTREAM_TEST_DATABASE_URL"]
async fn api_records_and_serves_history() {
    let db = isolated_test_pool().await;
    let api = Api::new(&db.pool).await;
    cache_video(&db.pool, VIDEO_A).await;

    let (_, state) = api
        .send(Method::GET, &format!("/api/v0/progress/{VIDEO_A}"))
        .await;
    assert!(state["progress"].is_null());

    assert_eq!(
        api.report("application/json", &report(VIDEO_A, 30.0, false))
            .await,
        StatusCode::NO_CONTENT
    );
    // `navigator.sendBeacon` sends plain text.
    assert_eq!(
        api.report("text/plain;charset=UTF-8", &report(VIDEO_B, 250.0, true))
            .await,
        StatusCode::NO_CONTENT
    );

    let (status, state) = api
        .send(Method::GET, &format!("/api/v0/progress/{VIDEO_A}"))
        .await;
    assert_eq!(status, StatusCode::OK);
    assert_eq!(state["progress"]["position_seconds"], 30.0);
    assert_eq!(state["progress"]["finished"], false);

    let (_, batch) = api
        .send(
            Method::GET,
            &format!("/api/v0/progress?ids={VIDEO_A},{VIDEO_B},{VIDEO_C}"),
        )
        .await;
    let mut batch: Vec<_> = batch
        .as_array()
        .unwrap()
        .iter()
        .map(|p| {
            (
                p["video_id"].as_str().unwrap().to_owned(),
                p["position_seconds"].clone(),
            )
        })
        .collect();
    batch.sort_by(|a, b| a.0.cmp(&b.0));
    assert_eq!(
        batch,
        [
            (VIDEO_A.to_owned(), json!(30.0)),
            (VIDEO_B.to_owned(), json!(200.0)),
        ],
        "positions are clamped to the duration"
    );

    let (status, page) = api.send(Method::GET, "/api/v0/history?limit=1").await;
    assert_eq!(status, StatusCode::OK, "{page}");
    assert_eq!(page["items"][0]["video"]["id"], VIDEO_B);
    assert_eq!(page["items"][0]["video"]["title"], VIDEO_B);
    assert!(page["items"][0]["video"]["channel"].is_null());
    assert!(
        page["items"][0]["video"]["thumbnails"][0]["url"]
            .as_str()
            .unwrap()
            .starts_with("/proxy/image/")
    );
    let continuation = page["continuation"].as_str().unwrap();
    let (_, next) = api
        .send(
            Method::GET,
            &format!("/api/v0/history?limit=1&continuation={continuation}"),
        )
        .await;
    assert_eq!(next["items"][0]["video"]["title"], "Video aaaaaaaaaaa");
    assert_eq!(next["items"][0]["video"]["channel"]["name"], "Alpha");
    assert_eq!(next["items"][0]["video"]["duration_seconds"], 200);
    assert!(next["continuation"].is_null());

    let (_, list) = api.send(Method::GET, "/api/v0/history/continue").await;
    let list = list.as_array().unwrap();
    assert_eq!(list.len(), 1);
    assert_eq!(list[0]["progress"]["video_id"], VIDEO_A);

    let (status, _) = api
        .send(Method::DELETE, &format!("/api/v0/history/{VIDEO_A}"))
        .await;
    assert_eq!(status, StatusCode::NO_CONTENT);
    let (_, page) = api.send(Method::GET, "/api/v0/history").await;
    assert_eq!(page["items"].as_array().unwrap().len(), 1);
    let (status, _) = api.send(Method::DELETE, "/api/v0/history").await;
    assert_eq!(status, StatusCode::NO_CONTENT);
    let (_, page) = api.send(Method::GET, "/api/v0/history").await;
    assert_eq!(page["items"].as_array().unwrap().len(), 0);
}

#[tokio::test]
#[ignore = "requires Docker or OXISTREAM_TEST_DATABASE_URL"]
async fn api_dismisses_from_continue_watching() {
    let db = isolated_test_pool().await;
    let api = Api::new(&db.pool).await;
    for id in [VIDEO_A, VIDEO_B] {
        assert_eq!(
            api.report("application/json", &report(id, 30.0, false))
                .await,
            StatusCode::NO_CONTENT
        );
    }

    let (status, _) = api
        .send(
            Method::DELETE,
            &format!("/api/v0/history/continue/{VIDEO_A}"),
        )
        .await;
    assert_eq!(status, StatusCode::NO_CONTENT);
    let (_, list) = api.send(Method::GET, "/api/v0/history/continue").await;
    let list = list.as_array().unwrap();
    assert_eq!(list.len(), 1);
    assert_eq!(list[0]["progress"]["video_id"], VIDEO_B);
    let (_, page) = api.send(Method::GET, "/api/v0/history").await;
    assert_eq!(
        page["items"].as_array().unwrap().len(),
        2,
        "history keeps it"
    );

    let (status, _) = api
        .send(
            Method::DELETE,
            &format!("/api/v0/history/continue/{VIDEO_C}"),
        )
        .await;
    assert_eq!(status, StatusCode::NO_CONTENT, "unknown videos succeed too");
    let (status, _) = api
        .send(Method::DELETE, "/api/v0/history/continue/nope")
        .await;
    assert_eq!(status, StatusCode::BAD_REQUEST);
}

#[tokio::test]
#[ignore = "requires Docker or OXISTREAM_TEST_DATABASE_URL"]
async fn api_pauses_history() {
    let db = isolated_test_pool().await;
    let api = Api::new(&db.pool).await;

    let (_, settings) = api.send(Method::GET, "/api/v0/history/settings").await;
    assert_eq!(settings["paused"], false);
    let (status, settings) = api
        .request(
            Method::PUT,
            "/api/v0/history/settings",
            Some(("application/json", json!({ "paused": true }).to_string())),
            true,
        )
        .await;
    assert_eq!(status, StatusCode::OK);
    assert_eq!(settings["paused"], true);

    assert_eq!(
        api.report("application/json", &report(VIDEO_A, 30.0, false))
            .await,
        StatusCode::NO_CONTENT,
        "accepted but ignored"
    );
    let (_, page) = api.send(Method::GET, "/api/v0/history").await;
    assert_eq!(page["items"].as_array().unwrap().len(), 0);
    let (_, settings) = api.send(Method::GET, "/api/v0/history/settings").await;
    assert_eq!(settings["paused"], true);
}

#[tokio::test]
#[ignore = "requires Docker or OXISTREAM_TEST_DATABASE_URL"]
async fn api_rejects_bad_input() {
    let db = isolated_test_pool().await;
    let api = Api::new(&db.pool).await;
    for body in [
        report("nope", 1.0, false),
        json!({ "video_id": VIDEO_A, "position_seconds": -1.0, "duration_seconds": 10.0 }),
        json!({ "video_id": VIDEO_A, "position_seconds": 1.0, "duration_seconds": 0.0 }),
        json!({ "video_id": VIDEO_A, "position_seconds": 1.0, "duration_seconds": 1e9 }),
        json!({ "video_id": VIDEO_A }),
    ] {
        assert_eq!(
            api.report("application/json", &body).await,
            StatusCode::BAD_REQUEST,
            "{body}"
        );
    }
    let many = (0..101)
        .map(|i| format!("aaaaaaa{i:04}"))
        .collect::<Vec<_>>()
        .join(",");
    for (method, uri) in [
        (Method::GET, "/api/v0/progress/nope".to_owned()),
        (Method::GET, "/api/v0/progress?ids=nope".to_owned()),
        (Method::GET, format!("/api/v0/progress?ids={many}")),
        (Method::DELETE, "/api/v0/history/nope".to_owned()),
        (Method::GET, "/api/v0/history?limit=0".to_owned()),
        (
            Method::GET,
            "/api/v0/history?continuation=garbage".to_owned(),
        ),
        (Method::GET, "/api/v0/history/continue?limit=51".to_owned()),
    ] {
        let (status, json) = api.send(method.clone(), &uri).await;
        assert_eq!(status, StatusCode::BAD_REQUEST, "{method} {uri}: {json}");
    }
    let (status, _) = api
        .request(
            Method::PUT,
            "/api/v0/history/settings",
            Some(("application/json", r#"{"paused":"yes"}"#.into())),
            true,
        )
        .await;
    assert!(status.is_client_error());
    let long = "a".repeat(201);
    let (status, _) = api
        .send(Method::GET, &format!("/api/v0/history?q={long}"))
        .await;
    assert_eq!(status, StatusCode::BAD_REQUEST);
}

/// Moves an entry `days` into the past.
async fn age(pool: &PgPool, user_id: Uuid, video_id: &str, days: i32) {
    sqlx::query(
        "UPDATE watch_progress SET updated_at = now() - make_interval(days => $3) \
         WHERE user_id = $1 AND video_id = $2",
    )
    .bind(user_id)
    .bind(video_id)
    .bind(days)
    .execute(pool)
    .await
    .unwrap();
}

async fn cache_titled(pool: &PgPool, id: &str, title: &str, channel: &str) {
    PgVideoRepository::new(pool.clone())
        .upsert(&VideoUpsert {
            id: id.to_owned(),
            title: title.to_owned(),
            channel_id: CHANNEL.into(),
            channel_name: channel.to_owned(),
            duration_seconds: Some(200),
            published_at: None,
            thumbnail_url: None,
        })
        .await
        .unwrap();
}

fn entry_ids(entries: &[oxistream_db::HistoryEntry]) -> Vec<&str> {
    entries
        .iter()
        .map(|e| e.progress.video_id.as_str())
        .collect()
}

#[tokio::test]
#[ignore = "requires Docker or OXISTREAM_TEST_DATABASE_URL"]
async fn searches_title_and_channel() {
    let db = isolated_test_pool().await;
    let history = PgHistoryRepository::new(db.pool.clone());
    let alice = user(&db.pool).await;
    cache_titled(&db.pool, VIDEO_A, "Rust in 100 Seconds", "Fireship").await;
    cache_titled(&db.pool, VIDEO_B, "Baking bread", "Kitchen 100%").await;
    for id in [VIDEO_A, VIDEO_B, VIDEO_C] {
        history
            .record(alice.id, &update(id, 30.0, false))
            .await
            .unwrap();
    }

    let search = async |q: &str| {
        entry_ids(&history.list(alice.id, Some(q), None, 10).await.unwrap())
            .into_iter()
            .map(str::to_owned)
            .collect::<Vec<_>>()
    };
    assert_eq!(search("rUST").await, [VIDEO_A]);
    assert_eq!(search("kitchen").await, [VIDEO_B]);
    assert_eq!(search("100").await, [VIDEO_B, VIDEO_A]);
    assert_eq!(search("100%").await, [VIDEO_B], "wildcards are literal");
    assert_eq!(search("_").await, Vec::<String>::new());
    assert_eq!(search("nothing").await, Vec::<String>::new());

    let first = history.list(alice.id, Some("100"), None, 1).await.unwrap();
    let next = history
        .list(alice.id, Some("100"), Some(&first[0].cursor()), 1)
        .await
        .unwrap();
    assert_eq!(
        entry_ids(&next),
        [VIDEO_A],
        "pages continue within the search"
    );
}

#[tokio::test]
#[ignore = "requires Docker or OXISTREAM_TEST_DATABASE_URL"]
async fn expired_entries_are_hidden_and_cleaned_up() {
    let db = isolated_test_pool().await;
    let settings = PgUserSettingsRepository::new(db.pool.clone());
    let alice = user(&db.pool).await;
    let bob = user(&db.pool).await;
    let carol = user(&db.pool).await;
    let history = PgHistoryRepository::new(db.pool.clone());
    for u in [&alice, &bob, &carol] {
        for id in [VIDEO_A, VIDEO_B, VIDEO_C] {
            history
                .record(u.id, &update(id, 30.0, false))
                .await
                .unwrap();
        }
        age(&db.pool, u.id, VIDEO_B, 10).await;
        age(&db.pool, u.id, VIDEO_C, 60).await;
    }
    settings
        .set_history_retention(alice.id, Some(7))
        .await
        .unwrap();
    settings
        .set_history_retention(bob.id, Some(365))
        .await
        .unwrap();

    // Bob keeps a year, but the operator allows 30 days; Carol keeps forever.
    let capped = PgHistoryRepository::new(db.pool.clone()).with_max_retention_days(Some(30));
    let listed = async |u: &User| {
        entry_ids(&capped.list(u.id, None, None, 10).await.unwrap())
            .into_iter()
            .map(str::to_owned)
            .collect::<Vec<_>>()
    };
    assert_eq!(listed(&alice).await, [VIDEO_A]);
    assert_eq!(listed(&bob).await, [VIDEO_A, VIDEO_B]);
    assert_eq!(listed(&carol).await, [VIDEO_A, VIDEO_B]);
    assert_eq!(
        history.list(carol.id, None, None, 10).await.unwrap().len(),
        3,
        "without a maximum, forever keeps everything"
    );
    let all = [VIDEO_A, VIDEO_B, VIDEO_C].map(str::to_owned);
    assert_eq!(capped.progress(alice.id, &all).await.unwrap().len(), 1);
    assert_eq!(
        capped.unfinished(alice.id, 10.0, 10).await.unwrap().len(),
        1
    );

    assert_eq!(history.delete_expired().await.unwrap(), 2, "only Alice's");
    assert_eq!(
        retention::cleanup(&db.pool, &capped).await.unwrap(),
        2,
        "Bob's and Carol's 60-day-old entries"
    );
    assert_eq!(retention::cleanup(&db.pool, &capped).await.unwrap(), 0);
    for u in [&alice, &bob, &carol] {
        let left = history.list(u.id, None, None, 10).await.unwrap();
        assert_eq!(left.len(), listed(u).await.len());
    }
}

#[tokio::test]
#[ignore = "requires Docker or OXISTREAM_TEST_DATABASE_URL"]
async fn api_searches_history() {
    let db = isolated_test_pool().await;
    let api = Api::new(&db.pool).await;
    cache_titled(&db.pool, VIDEO_A, "Rust in 100 Seconds", "Fireship").await;
    cache_titled(&db.pool, VIDEO_B, "Baking bread", "Kitchen").await;
    for id in [VIDEO_A, VIDEO_B] {
        api.report("application/json", &report(id, 30.0, false))
            .await;
    }
    let (status, page) = api.send(Method::GET, "/api/v0/history?q=%20fire%20").await;
    assert_eq!(status, StatusCode::OK, "{page}");
    let items = page["items"].as_array().unwrap();
    assert_eq!(items.len(), 1);
    assert_eq!(items[0]["video"]["id"], VIDEO_A);
    let (_, page) = api.send(Method::GET, "/api/v0/history?q=").await;
    assert_eq!(
        page["items"].as_array().unwrap().len(),
        2,
        "empty q lists all"
    );
}

#[tokio::test]
#[ignore = "requires Docker or OXISTREAM_TEST_DATABASE_URL"]
async fn api_sets_retention_up_to_the_maximum() {
    let db = isolated_test_pool().await;
    let api = Api::with_max_retention(&db.pool, Retention::ThreeMonths).await;
    let put = async |body: Value| {
        api.request(
            Method::PUT,
            "/api/v0/history/settings",
            Some(("application/json", body.to_string())),
            true,
        )
        .await
    };

    let (_, instance) = api.send(Method::GET, "/api/v0/instance").await;
    assert_eq!(instance["history_max_retention"], "90d");
    let (_, settings) = api.send(Method::GET, "/api/v0/history/settings").await;
    assert_eq!(settings, json!({ "paused": false, "retention": "90d" }));

    let (status, settings) = put(json!({ "retention": "7d" })).await;
    assert_eq!(status, StatusCode::OK, "{settings}");
    assert_eq!(settings, json!({ "paused": false, "retention": "7d" }));
    let (_, settings) = put(json!({ "paused": true })).await;
    assert_eq!(
        settings,
        json!({ "paused": true, "retention": "7d" }),
        "kept"
    );

    for value in ["180d", "forever"] {
        let (status, body) = put(json!({ "retention": value })).await;
        assert_eq!(status, StatusCode::UNPROCESSABLE_ENTITY, "{value}");
        assert_eq!(body["fields"]["retention"], "invalid");
    }
    let (status, _) = put(json!({ "retention": "1y" })).await;
    assert!(status.is_client_error());

    // A value stored under a higher maximum acts as the current one.
    PgUserSettingsRepository::new(db.pool.clone())
        .set_history_retention(api.user_id, Some(365))
        .await
        .unwrap();
    let (_, settings) = api.send(Method::GET, "/api/v0/history/settings").await;
    assert_eq!(settings["retention"], "90d");
}
