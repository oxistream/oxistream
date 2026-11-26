//! Needs `OXISTREAM_TEST_DATABASE_URL` or a Docker daemon; run with `cargo test -- --include-ignored`.

mod common;

use axum::Router;
use axum::body::Body;
use axum::http::{Method, Request, StatusCode, header};
use common::{isolated_test_pool, state_with_pool, unique_suffix};
use http_body_util::BodyExt;
use oxistream_db::{
    LikeRepository, NewSession, NewUser, PgLikeRepository, PgPool, PgSessionRepository,
    PgUserRepository, PgVideoRepository, SessionRepository, User, UserRepository, VideoRepository,
    VideoUpsert,
};
use oxistream_server::auth::cookie;
use serde_json::Value;
use time::OffsetDateTime;
use tower::ServiceExt;

const VIDEOS: [&str; 3] = ["vvvvvvvvvv0", "vvvvvvvvvv1", "vvvvvvvvvv2"];

async fn user(pool: &PgPool) -> User {
    PgUserRepository::new(pool.clone())
        .create(&NewUser {
            username: format!("u_{}", unique_suffix()),
            password_hash: None,
        })
        .await
        .unwrap()
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
                duration_seconds: Some(42),
                published_at: None,
                thumbnail_url: None,
            })
            .await
            .unwrap();
    }
}

async fn liked_ids(repo: &PgLikeRepository, user: &User) -> Vec<String> {
    repo.list(user.id, None, 10)
        .await
        .unwrap()
        .into_iter()
        .map(|l| l.video.id)
        .collect()
}

#[tokio::test]
#[ignore = "requires Docker or OXISTREAM_TEST_DATABASE_URL"]
async fn likes_are_per_user_and_newest_first() {
    let db = isolated_test_pool().await;
    cache_videos(&db.pool).await;
    let repo = PgLikeRepository::new(db.pool.clone());
    let alice = user(&db.pool).await;
    let bob = user(&db.pool).await;

    for id in VIDEOS {
        let (_, created) = repo.like(alice.id, id).await.unwrap();
        assert!(created);
    }
    let (first_at, created) = repo.like(alice.id, VIDEOS[0]).await.unwrap();
    assert!(!created, "liking twice is a no-op");
    assert_eq!(
        repo.liked_at(alice.id, VIDEOS[0]).await.unwrap(),
        Some(first_at),
        "keeps the original date"
    );
    assert_eq!(
        liked_ids(&repo, &alice).await,
        [VIDEOS[2], VIDEOS[1], VIDEOS[0]]
    );
    assert_eq!(repo.count(alice.id).await.unwrap(), 3);
    assert_eq!(repo.count(bob.id).await.unwrap(), 0);
    assert_eq!(repo.liked_at(bob.id, VIDEOS[0]).await.unwrap(), None);

    let page = repo.list(alice.id, None, 2).await.unwrap();
    assert_eq!(page.len(), 2);
    let rest = repo
        .list(alice.id, Some(&page[1].cursor()), 2)
        .await
        .unwrap();
    assert_eq!(rest.len(), 1);
    assert_eq!(rest[0].video.id, VIDEOS[0]);
    assert_eq!(rest[0].video.title, "Video vvvvvvvvvv0");

    assert!(repo.unlike(alice.id, VIDEOS[1]).await.unwrap());
    assert!(!repo.unlike(alice.id, VIDEOS[1]).await.unwrap());
    assert!(!repo.unlike(bob.id, VIDEOS[0]).await.unwrap());
    assert_eq!(liked_ids(&repo, &alice).await, [VIDEOS[2], VIDEOS[0]]);
}

#[tokio::test]
#[ignore = "requires Docker or OXISTREAM_TEST_DATABASE_URL"]
async fn deleting_the_account_deletes_its_likes() {
    let db = isolated_test_pool().await;
    cache_videos(&db.pool).await;
    let repo = PgLikeRepository::new(db.pool.clone());
    let alice = user(&db.pool).await;
    repo.like(alice.id, VIDEOS[0]).await.unwrap();
    sqlx::query("DELETE FROM users WHERE id = $1")
        .bind(alice.id)
        .execute(&db.pool)
        .await
        .unwrap();
    let left: i64 = sqlx::query_scalar("SELECT count(*) FROM video_likes")
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
    let like = "/api/v0/likes/vvvvvvvvvv0";
    for (method, uri) in [
        (Method::GET, "/api/v0/likes"),
        (Method::GET, like),
        (Method::PUT, like),
        (Method::DELETE, like),
    ] {
        let (status, json) = api.send(method.clone(), uri, None).await;
        assert_eq!(status, StatusCode::UNAUTHORIZED, "{method} {uri}: {json}");
    }
}

#[tokio::test]
#[ignore = "requires Docker or OXISTREAM_TEST_DATABASE_URL"]
async fn api_likes_lists_and_unlikes() {
    let db = isolated_test_pool().await;
    cache_videos(&db.pool).await;
    let api = Api::new(&db.pool);
    let alice = Api::session(&db.pool).await;
    let bob = Api::session(&db.pool).await;
    let alice = Some(alice.as_str());
    let bob = Some(bob.as_str());

    let (status, json) = api
        .send(Method::GET, "/api/v0/likes/vvvvvvvvvv0", alice)
        .await;
    assert_eq!(status, StatusCode::OK);
    assert_eq!(json["liked"], false);
    assert!(json["liked_at"].is_null());

    for id in VIDEOS {
        let (status, json) = api
            .send(Method::PUT, &format!("/api/v0/likes/{id}"), alice)
            .await;
        assert_eq!(status, StatusCode::CREATED, "{json}");
        assert_eq!(json["liked"], true);
    }
    let (status, json) = api
        .send(Method::PUT, "/api/v0/likes/vvvvvvvvvv0", alice)
        .await;
    assert_eq!(status, StatusCode::OK);
    assert!(json["liked_at"].is_string());

    let (status, page) = api.send(Method::GET, "/api/v0/likes?limit=2", alice).await;
    assert_eq!(status, StatusCode::OK, "{page}");
    assert_eq!(page["total"], 3);
    let ids: Vec<&str> = page["items"]
        .as_array()
        .unwrap()
        .iter()
        .map(|i| i["video"]["id"].as_str().unwrap())
        .collect();
    assert_eq!(ids, [VIDEOS[2], VIDEOS[1]]);
    assert_eq!(page["items"][0]["video"]["channel"]["name"], "Alpha");
    let continuation = page["continuation"].as_str().unwrap();
    let (_, rest) = api
        .send(
            Method::GET,
            &format!("/api/v0/likes?limit=2&continuation={continuation}"),
            alice,
        )
        .await;
    assert_eq!(rest["items"][0]["video"]["id"], VIDEOS[0]);
    assert!(rest["continuation"].is_null());

    let (_, json) = api.send(Method::GET, "/api/v0/likes", bob).await;
    assert_eq!(json["total"], 0, "likes are per user");

    let (status, _) = api
        .send(Method::DELETE, "/api/v0/likes/vvvvvvvvvv1", alice)
        .await;
    assert_eq!(status, StatusCode::NO_CONTENT);
    let (status, _) = api
        .send(Method::DELETE, "/api/v0/likes/vvvvvvvvvv1", alice)
        .await;
    assert_eq!(status, StatusCode::NO_CONTENT, "unliking twice succeeds");
    let (_, json) = api
        .send(Method::GET, "/api/v0/likes/vvvvvvvvvv1", alice)
        .await;
    assert_eq!(json["liked"], false);
    let (_, json) = api.send(Method::GET, "/api/v0/likes", alice).await;
    assert_eq!(json["total"], 2);
}

#[tokio::test]
#[ignore = "requires Docker or OXISTREAM_TEST_DATABASE_URL"]
async fn api_rejects_bad_input() {
    let db = isolated_test_pool().await;
    let api = Api::new(&db.pool);
    let token = Api::session(&db.pool).await;
    let token = Some(token.as_str());
    for (method, uri) in [
        (Method::GET, "/api/v0/likes/not-a-video!"),
        (Method::PUT, "/api/v0/likes/short"),
        (Method::DELETE, "/api/v0/likes/short"),
        (Method::GET, "/api/v0/likes?limit=0"),
        (Method::GET, "/api/v0/likes?limit=101"),
        (Method::GET, "/api/v0/likes?continuation=nope"),
    ] {
        let (status, json) = api.send(method.clone(), uri, token).await;
        assert_eq!(status, StatusCode::BAD_REQUEST, "{method} {uri}: {json}");
    }
}
