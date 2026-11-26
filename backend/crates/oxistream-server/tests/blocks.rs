//! Needs `OXISTREAM_TEST_DATABASE_URL` or a Docker daemon; run with `cargo test -- --include-ignored`.

mod common;

use axum::Router;
use axum::body::Body;
use axum::http::{Method, Request, StatusCode, header};
use common::{isolated_test_pool, state_with_pool, unique_suffix};
use http_body_util::BodyExt;
use oxistream_db::{
    ChannelBlockRepository, Image, ImportedChannel, NewChannelBlock, NewSession, NewUser,
    PgChannelBlockRepository, PgPool, PgSessionRepository, PgSubscriptionRepository,
    PgUserRepository, SessionRepository, SubscriptionRepository, User, UserRepository,
};
use oxistream_server::auth::cookie;
use serde_json::Value;
use time::OffsetDateTime;
use tower::ServiceExt;

const ALPHA: &str = "UCaaaaaaaaaaaaaaaaaaaaaa";
const BETA: &str = "UCbbbbbbbbbbbbbbbbbbbbbb";

async fn user(pool: &PgPool) -> User {
    PgUserRepository::new(pool.clone())
        .create(&NewUser {
            username: format!("u_{}", unique_suffix()),
            password_hash: None,
        })
        .await
        .unwrap()
}

fn new_block(channel_id: &str, name: &str) -> NewChannelBlock {
    NewChannelBlock {
        channel_id: channel_id.into(),
        channel_name: name.into(),
        avatar: vec![Image {
            url: format!("https://yt3.ggpht.com/{name}=s88"),
            width: 88,
            height: 88,
        }],
    }
}

async fn listed(repo: &PgChannelBlockRepository, user: &User) -> Vec<String> {
    repo.list(user.id)
        .await
        .unwrap()
        .into_iter()
        .map(|b| b.channel_id)
        .collect()
}

#[tokio::test]
#[ignore = "requires Docker or OXISTREAM_TEST_DATABASE_URL"]
async fn blocks_are_per_user_and_newest_first() {
    let db = isolated_test_pool().await;
    let repo = PgChannelBlockRepository::new(db.pool.clone());
    let alice = user(&db.pool).await;
    let bob = user(&db.pool).await;

    let (first, created) = repo
        .block(alice.id, &new_block(ALPHA, "Alpha"))
        .await
        .unwrap();
    assert!(created);
    assert_eq!(first.channel_name, "Alpha");
    assert_eq!(first.avatar.len(), 1);
    repo.block(alice.id, &new_block(BETA, "Beta"))
        .await
        .unwrap();

    let (again, created) = repo
        .block(alice.id, &new_block(ALPHA, "Alpha renamed"))
        .await
        .unwrap();
    assert!(!created, "blocking twice is a no-op");
    assert_eq!(
        again.created_at, first.created_at,
        "keeps the original date"
    );
    assert_eq!(again.channel_name, "Alpha renamed");

    assert_eq!(listed(&repo, &alice).await, [BETA, ALPHA]);
    assert_eq!(listed(&repo, &bob).await, Vec::<String>::new());
    let ids = repo.blocked_ids(alice.id).await.unwrap();
    assert!(ids.contains(ALPHA) && ids.contains(BETA) && ids.len() == 2);
    assert!(repo.blocked_ids(bob.id).await.unwrap().is_empty());
    assert!(repo.get(bob.id, ALPHA).await.unwrap().is_none());

    assert!(repo.unblock(alice.id, ALPHA).await.unwrap());
    assert!(!repo.unblock(alice.id, ALPHA).await.unwrap());
    assert!(!repo.unblock(bob.id, BETA).await.unwrap());
    assert_eq!(listed(&repo, &alice).await, [BETA]);
}

#[tokio::test]
#[ignore = "requires Docker or OXISTREAM_TEST_DATABASE_URL"]
async fn blocking_unsubscribes_and_subscribing_unblocks() {
    let db = isolated_test_pool().await;
    let blocks = PgChannelBlockRepository::new(db.pool.clone());
    let subscriptions = PgSubscriptionRepository::new(db.pool.clone());
    let alice = user(&db.pool).await;
    let bob = user(&db.pool).await;

    subscriptions.subscribe(alice.id, ALPHA).await.unwrap();
    subscriptions.subscribe(bob.id, ALPHA).await.unwrap();
    blocks
        .block(alice.id, &new_block(ALPHA, "Alpha"))
        .await
        .unwrap();
    assert!(!subscriptions.is_subscribed(alice.id, ALPHA).await.unwrap());
    assert!(
        subscriptions.is_subscribed(bob.id, ALPHA).await.unwrap(),
        "others keep their subscription"
    );

    subscriptions.subscribe(alice.id, ALPHA).await.unwrap();
    assert!(blocks.get(alice.id, ALPHA).await.unwrap().is_none());

    blocks
        .block(alice.id, &new_block(ALPHA, "Alpha"))
        .await
        .unwrap();
    blocks
        .block(alice.id, &new_block(BETA, "Beta"))
        .await
        .unwrap();
    subscriptions
        .import(
            alice.id,
            &[ImportedChannel {
                id: BETA.into(),
                name: None,
            }],
        )
        .await
        .unwrap();
    assert_eq!(listed(&blocks, &alice).await, [ALPHA], "import unblocks");
}

#[tokio::test]
#[ignore = "requires Docker or OXISTREAM_TEST_DATABASE_URL"]
async fn deleting_the_account_deletes_its_blocks() {
    let db = isolated_test_pool().await;
    let repo = PgChannelBlockRepository::new(db.pool.clone());
    let alice = user(&db.pool).await;
    repo.block(alice.id, &new_block(ALPHA, "Alpha"))
        .await
        .unwrap();
    sqlx::query("DELETE FROM users WHERE id = $1")
        .bind(alice.id)
        .execute(&db.pool)
        .await
        .unwrap();
    let left: i64 = sqlx::query_scalar("SELECT count(*) FROM channel_blocks")
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

    async fn session(pool: &PgPool) -> (User, String) {
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
        (user, token)
    }

    async fn send(&self, method: Method, uri: &str, token: Option<&str>) -> (StatusCode, Value) {
        let mut req = Request::builder()
            .method(method)
            .uri(uri)
            .header(header::HOST, "tube.test")
            .header(header::ORIGIN, "http://tube.test");
        if let Some(token) = token {
            req = req.header(header::COOKIE, format!("oxistream_session={token}"));
        }
        let response = self
            .app
            .clone()
            .oneshot(req.body(Body::empty()).unwrap())
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

#[tokio::test]
#[ignore = "requires Docker or OXISTREAM_TEST_DATABASE_URL"]
async fn api_requires_login() {
    let db = isolated_test_pool().await;
    let api = Api::new(&db.pool);
    let block = format!("/api/v0/blocks/{ALPHA}");
    for (method, uri) in [
        (Method::GET, "/api/v0/blocks"),
        (Method::PUT, block.as_str()),
        (Method::DELETE, block.as_str()),
    ] {
        let (status, json) = api.send(method.clone(), uri, None).await;
        assert_eq!(status, StatusCode::UNAUTHORIZED, "{method} {uri}: {json}");
    }
}

#[tokio::test]
#[ignore = "requires Docker or OXISTREAM_TEST_DATABASE_URL"]
async fn api_lists_and_unblocks() {
    let db = isolated_test_pool().await;
    let api = Api::new(&db.pool);
    let repo = PgChannelBlockRepository::new(db.pool.clone());
    let (alice, token) = Api::session(&db.pool).await;
    let token = Some(token.as_str());

    let (status, json) = api.send(Method::GET, "/api/v0/blocks", token).await;
    assert_eq!(status, StatusCode::OK);
    assert_eq!(json, Value::Array(Vec::new()));

    repo.block(alice.id, &new_block(ALPHA, "Alpha"))
        .await
        .unwrap();
    // Known blocks are answered from the database, without asking YouTube.
    let (status, json) = api
        .send(Method::PUT, &format!("/api/v0/blocks/{ALPHA}"), token)
        .await;
    assert_eq!(status, StatusCode::OK, "{json}");
    assert_eq!(json["id"], ALPHA);
    assert_eq!(json["name"], "Alpha");
    assert!(json["blocked_at"].is_string());

    let (status, json) = api.send(Method::GET, "/api/v0/blocks", token).await;
    assert_eq!(status, StatusCode::OK);
    let items = json.as_array().unwrap();
    assert_eq!(items.len(), 1);
    let avatar = items[0]["avatar"][0]["url"].as_str().unwrap();
    assert!(avatar.starts_with("/proxy/image/"), "{avatar}");

    let (status, _) = api
        .send(Method::DELETE, &format!("/api/v0/blocks/{ALPHA}"), token)
        .await;
    assert_eq!(status, StatusCode::NO_CONTENT);
    let (status, _) = api
        .send(Method::DELETE, &format!("/api/v0/blocks/{ALPHA}"), token)
        .await;
    assert_eq!(status, StatusCode::NO_CONTENT, "unblocking twice succeeds");
    assert_eq!(repo.list(alice.id).await.unwrap().len(), 0);

    let (status, _) = api
        .send(Method::PUT, "/api/v0/blocks/not-a-channel", token)
        .await;
    assert_eq!(status, StatusCode::BAD_REQUEST);
}

#[tokio::test]
#[ignore = "requires Docker or OXISTREAM_TEST_DATABASE_URL"]
async fn api_hides_blocked_channel_tabs() {
    let db = isolated_test_pool().await;
    let api = Api::new(&db.pool);
    let (alice, token) = Api::session(&db.pool).await;
    PgChannelBlockRepository::new(db.pool.clone())
        .block(alice.id, &new_block(ALPHA, "Alpha"))
        .await
        .unwrap();
    // Answered without asking YouTube, so no network is needed.
    for tab in ["videos", "shorts", "live", "playlists", "search?q=x"] {
        let (status, json) = api
            .send(
                Method::GET,
                &format!("/api/v0/channels/{ALPHA}/{tab}"),
                Some(&token),
            )
            .await;
        assert_eq!(status, StatusCode::OK, "{tab}: {json}");
        assert_eq!(json["items"], Value::Array(Vec::new()), "{tab}");
        assert!(json["continuation"].is_null(), "{tab}");
    }
}
