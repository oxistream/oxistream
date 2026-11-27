//! Needs `OXISTREAM_TEST_DATABASE_URL` or a Docker daemon; run with `cargo test -- --include-ignored`.

mod common;

use axum::Router;
use axum::body::Body;
use axum::http::{Method, Request, StatusCode, header};
use common::{isolated_test_pool, state_with_pool, unique_suffix};
use http_body_util::BodyExt;
use oxistream_db::{
    NewSession, NewUser, PgPool, PgQueueRepository, PgSessionRepository, PgUserRepository,
    QueueEntry, QueueOp, QueuePosition, QueueRepository, SessionRepository, User, UserRepository,
};
use oxistream_server::auth::cookie;
use serde_json::{Value, json};
use time::OffsetDateTime;
use tower::ServiceExt;

const VIDEOS: [&str; 3] = ["vvvvvvvvvv0", "vvvvvvvvvv1", "vvvvvvvvvv2"];
const CHANNEL: &str = "UCaaaaaaaaaaaaaaaaaaaaaa";

async fn user(pool: &PgPool) -> User {
    PgUserRepository::new(pool.clone())
        .create(&NewUser {
            username: format!("u_{}", unique_suffix()),
            password_hash: None,
        })
        .await
        .unwrap()
}

fn entry(key: &str, video_id: &str) -> QueueEntry {
    QueueEntry {
        key: key.into(),
        video_id: video_id.into(),
        title: format!("Video {video_id}"),
        channel_id: Some(CHANNEL.into()),
        channel_name: Some("Alpha".into()),
        duration_seconds: Some(42),
    }
}

fn add(entries: Vec<QueueEntry>, position: QueuePosition) -> QueueOp {
    QueueOp::Add {
        entries,
        position,
        skip_queued: false,
    }
}

fn keys(queue: &oxistream_db::Queue) -> Vec<&str> {
    queue.entries.iter().map(|e| e.key.as_str()).collect()
}

#[tokio::test]
#[ignore = "requires Docker or OXISTREAM_TEST_DATABASE_URL"]
async fn repository_keeps_order_per_user_and_counts_revisions() {
    let db = isolated_test_pool().await;
    let repo = PgQueueRepository::new(db.pool.clone());
    let alice = user(&db.pool).await;
    let bob = user(&db.pool).await;

    let empty = repo.get(alice.id).await.unwrap();
    assert_eq!((empty.revision, empty.entries.len()), (0, 0));

    let queue = repo
        .apply(
            alice.id,
            &add(
                vec![entry("a", VIDEOS[0]), entry("b", VIDEOS[1])],
                QueuePosition::End,
            ),
            200,
        )
        .await
        .unwrap();
    assert_eq!(keys(&queue), ["a", "b"]);
    assert_eq!(queue.revision, 1);
    let queue = repo
        .apply(
            alice.id,
            &add(vec![entry("c", VIDEOS[2])], QueuePosition::Start),
            200,
        )
        .await
        .unwrap();
    assert_eq!(keys(&queue), ["c", "a", "b"]);
    assert_eq!(queue.revision, 2);

    let stored = repo.get(alice.id).await.unwrap();
    assert_eq!(stored, queue, "stored as returned");
    assert_eq!(stored.entries[0], entry("c", VIDEOS[2]));
    assert_eq!(repo.get(bob.id).await.unwrap().entries.len(), 0);

    let moved = repo
        .apply(
            alice.id,
            &QueueOp::Move {
                key: "c".into(),
                index: 5,
            },
            200,
        )
        .await
        .unwrap();
    assert_eq!(keys(&moved), ["a", "b", "c"]);

    let remove = QueueOp::Remove { key: "a".into() };
    let removed = repo.apply(alice.id, &remove, 200).await.unwrap();
    assert_eq!(keys(&removed), ["b", "c"]);
    let again = repo.apply(alice.id, &remove, 200).await.unwrap();
    assert_eq!(again, removed, "a second consumer changes nothing");

    let cleared = repo.apply(alice.id, &QueueOp::Clear, 200).await.unwrap();
    assert_eq!(cleared.entries.len(), 0);
    assert_eq!(cleared.revision, removed.revision + 1);
}

#[tokio::test]
#[ignore = "requires Docker or OXISTREAM_TEST_DATABASE_URL"]
async fn concurrent_changes_are_all_kept() {
    let db = isolated_test_pool().await;
    let repo = PgQueueRepository::new(db.pool.clone());
    let alice = user(&db.pool).await;
    let ops: Vec<_> = (0..10)
        .map(|i| add(vec![entry(&format!("k{i}"), VIDEOS[0])], QueuePosition::End))
        .collect();
    let results =
        futures_util::future::join_all(ops.iter().map(|op| repo.apply(alice.id, op, 200))).await;
    assert!(results.iter().all(Result::is_ok));
    let queue = repo.get(alice.id).await.unwrap();
    assert_eq!(queue.entries.len(), 10);
    assert_eq!(queue.revision, 10);
}

#[tokio::test]
#[ignore = "requires Docker or OXISTREAM_TEST_DATABASE_URL"]
async fn deleting_the_account_deletes_its_queue() {
    let db = isolated_test_pool().await;
    let repo = PgQueueRepository::new(db.pool.clone());
    let alice = user(&db.pool).await;
    repo.apply(
        alice.id,
        &add(vec![entry("a", VIDEOS[0])], QueuePosition::End),
        200,
    )
    .await
    .unwrap();
    sqlx::query("DELETE FROM users WHERE id = $1")
        .bind(alice.id)
        .execute(&db.pool)
        .await
        .unwrap();
    let left: i64 = sqlx::query_scalar("SELECT count(*) FROM queue_entries")
        .fetch_one(&db.pool)
        .await
        .unwrap();
    assert_eq!(left, 0);
}

struct Api {
    app: Router,
}

impl Api {
    fn new(pool: &PgPool) -> Self {
        Self {
            app: oxistream_server::app(state_with_pool(pool.clone())),
        }
    }

    async fn session(pool: &PgPool) -> String {
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
        token
    }

    async fn send(
        &self,
        method: Method,
        uri: &str,
        token: Option<&str>,
        body: Option<Value>,
    ) -> (StatusCode, Value) {
        let mut req = Request::builder()
            .method(method)
            .uri(uri)
            .header(header::HOST, "tube.test")
            .header(header::ORIGIN, "http://tube.test");
        if let Some(token) = token {
            req = req.header(header::COOKIE, format!("oxistream_session={token}"));
        }
        let body = match body {
            Some(body) => {
                req = req.header(header::CONTENT_TYPE, "application/json");
                Body::from(body.to_string())
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

fn new_entry(key: &str, video_id: &str) -> Value {
    json!({
        "key": key,
        "video_id": video_id,
        "title": format!("Video {video_id}"),
        "channel_id": CHANNEL,
        "channel_name": "Alpha",
        "duration_seconds": 42,
    })
}

fn video_ids(queue: &Value) -> Vec<&str> {
    queue["entries"]
        .as_array()
        .unwrap()
        .iter()
        .map(|e| e["video"]["id"].as_str().unwrap())
        .collect()
}

#[tokio::test]
#[ignore = "requires Docker or OXISTREAM_TEST_DATABASE_URL"]
async fn api_requires_login() {
    let db = isolated_test_pool().await;
    let api = Api::new(&db.pool);
    for (method, uri) in [
        (Method::GET, "/api/v0/queue"),
        (Method::DELETE, "/api/v0/queue"),
        (Method::POST, "/api/v0/queue/entries"),
        (Method::DELETE, "/api/v0/queue/entries/a"),
        (Method::PUT, "/api/v0/queue/entries/a/position"),
    ] {
        let (status, json) = api.send(method.clone(), uri, None, None).await;
        assert_eq!(status, StatusCode::UNAUTHORIZED, "{method} {uri}: {json}");
    }
}

#[tokio::test]
#[ignore = "requires Docker or OXISTREAM_TEST_DATABASE_URL"]
async fn api_adds_moves_removes_and_merges() {
    let db = isolated_test_pool().await;
    let api = Api::new(&db.pool);
    let alice = Api::session(&db.pool).await;
    let bob = Api::session(&db.pool).await;
    let alice = Some(alice.as_str());
    let bob = Some(bob.as_str());

    let (status, queue) = api.send(Method::GET, "/api/v0/queue", alice, None).await;
    assert_eq!(status, StatusCode::OK);
    assert_eq!(queue, json!({ "revision": 0, "entries": [] }));

    let body = json!({ "entries": [new_entry("a", VIDEOS[0]), new_entry("b", VIDEOS[1])] });
    let (status, queue) = api
        .send(Method::POST, "/api/v0/queue/entries", alice, Some(body))
        .await;
    assert_eq!(status, StatusCode::OK, "{queue}");
    assert_eq!(video_ids(&queue), [VIDEOS[0], VIDEOS[1]]);
    let first = &queue["entries"][0];
    assert_eq!(first["key"], "a");
    assert_eq!(first["video"]["title"], "Video vvvvvvvvvv0");
    assert_eq!(first["video"]["channel"]["name"], "Alpha");
    assert_eq!(first["video"]["duration_seconds"], 42);
    assert!(
        first["video"]["thumbnails"][0]["url"]
            .as_str()
            .unwrap()
            .starts_with("/proxy/image/")
    );

    let body = json!({ "entries": [new_entry("c", VIDEOS[2])], "position": "start" });
    let (_, queue) = api
        .send(Method::POST, "/api/v0/queue/entries", alice, Some(body))
        .await;
    assert_eq!(video_ids(&queue), [VIDEOS[2], VIDEOS[0], VIDEOS[1]]);
    assert_eq!(queue["revision"], 2);

    let (status, queue) = api
        .send(
            Method::PUT,
            "/api/v0/queue/entries/c/position",
            alice,
            Some(json!({ "index": 2 })),
        )
        .await;
    assert_eq!(status, StatusCode::OK);
    assert_eq!(video_ids(&queue), [VIDEOS[0], VIDEOS[1], VIDEOS[2]]);

    for _ in 0..2 {
        let (status, queue) = api
            .send(Method::DELETE, "/api/v0/queue/entries/a", alice, None)
            .await;
        assert_eq!(status, StatusCode::OK, "removing twice succeeds");
        assert_eq!(video_ids(&queue), [VIDEOS[1], VIDEOS[2]]);
    }

    // A queue from before signing in: queued videos are left out.
    let body = json!({
        "entries": [new_entry("x", VIDEOS[1]), new_entry("y", VIDEOS[0])],
        "skip_queued": true,
    });
    let (_, queue) = api
        .send(Method::POST, "/api/v0/queue/entries", alice, Some(body))
        .await;
    assert_eq!(video_ids(&queue), [VIDEOS[1], VIDEOS[2], VIDEOS[0]]);

    let (_, other) = api.send(Method::GET, "/api/v0/queue", bob, None).await;
    assert_eq!(video_ids(&other), Vec::<&str>::new(), "queues are per user");

    let (status, queue) = api.send(Method::DELETE, "/api/v0/queue", alice, None).await;
    assert_eq!(status, StatusCode::OK);
    assert_eq!(video_ids(&queue), Vec::<&str>::new());
}

#[tokio::test]
#[ignore = "requires Docker or OXISTREAM_TEST_DATABASE_URL"]
async fn api_validates_entries_and_keeps_the_limit() {
    let db = isolated_test_pool().await;
    let api = Api::new(&db.pool);
    let alice = Api::session(&db.pool).await;
    let alice = Some(alice.as_str());

    let mut bad_channel = new_entry("a", VIDEOS[0]);
    bad_channel["channel_id"] = json!("nope");
    let mut no_title = new_entry("a", VIDEOS[0]);
    no_title["title"] = json!("  ");
    for (entries, what) in [
        (json!([new_entry("a", "short")]), "video id"),
        (json!([new_entry("a/b", VIDEOS[0])]), "key"),
        (json!([new_entry(&"k".repeat(65), VIDEOS[0])]), "long key"),
        (json!([bad_channel]), "channel id"),
        (json!([no_title]), "title"),
        (
            json!([new_entry("a", VIDEOS[0]), new_entry("a", VIDEOS[1])]),
            "duplicate keys",
        ),
    ] {
        let (status, json) = api
            .send(
                Method::POST,
                "/api/v0/queue/entries",
                alice,
                Some(json!({ "entries": entries })),
            )
            .await;
        assert_eq!(status, StatusCode::BAD_REQUEST, "{what}: {json}");
    }
    let (status, _) = api
        .send(Method::DELETE, "/api/v0/queue/entries/a%20b", alice, None)
        .await;
    assert_eq!(status, StatusCode::BAD_REQUEST);

    let many: Vec<Value> = (0..201)
        .map(|i| new_entry(&format!("k{i}"), VIDEOS[0]))
        .collect();
    let (status, _) = api
        .send(
            Method::POST,
            "/api/v0/queue/entries",
            alice,
            Some(json!({ "entries": many })),
        )
        .await;
    assert_eq!(status, StatusCode::BAD_REQUEST, "too many at once");

    let full: Vec<Value> = (0..200)
        .map(|i| new_entry(&format!("k{i}"), VIDEOS[0]))
        .collect();
    let (status, _) = api
        .send(
            Method::POST,
            "/api/v0/queue/entries",
            alice,
            Some(json!({ "entries": full })),
        )
        .await;
    assert_eq!(status, StatusCode::OK);
    let (_, queue) = api
        .send(
            Method::POST,
            "/api/v0/queue/entries",
            alice,
            Some(json!({ "entries": [new_entry("top", VIDEOS[1])], "position": "start" })),
        )
        .await;
    let entries = queue["entries"].as_array().unwrap();
    assert_eq!(entries.len(), 200);
    assert_eq!(entries[0]["key"], "top");
    assert_eq!(entries[199]["key"], "k198", "the last one fell off");
}
