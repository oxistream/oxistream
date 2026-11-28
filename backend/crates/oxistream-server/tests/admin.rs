//! Needs `OXISTREAM_TEST_DATABASE_URL` or a Docker daemon; run with `cargo test -- --include-ignored`.

mod common;

use axum::Router;
use axum::body::Body;
use axum::http::{Method, Request, StatusCode, header};
use common::{isolated_test_pool, state_with_pool, unique_suffix};
use http_body_util::BodyExt;
use oxistream_db::{
    AdminUpdate, AdvisoryLock, Identity, LockKey, NewUser, PgPool, PgStatsRepository,
    PgUserRepository, Role, StatsRepository, UserRepository, Uuid,
};
use serde_json::{Value, json};
use tower::ServiceExt;

const PASSWORD: &str = "correct horse battery";

struct Reply {
    status: StatusCode,
    set_cookie: Option<String>,
    json: Value,
}

async fn send(
    app: &Router,
    method: Method,
    uri: &str,
    body: Option<Value>,
    token: Option<&str>,
) -> Reply {
    let mut req = Request::builder()
        .method(method)
        .uri(uri)
        .header(header::HOST, "tube.test")
        .header(header::ORIGIN, "http://tube.test");
    if let Some(token) = token {
        req = req.header(header::COOKIE, format!("oxistream_session={token}"));
    }
    let req = match body {
        Some(body) => req
            .header(header::CONTENT_TYPE, "application/json")
            .body(Body::from(body.to_string())),
        None => req.body(Body::empty()),
    }
    .unwrap();
    let response = app.clone().oneshot(req).await.unwrap();
    let status = response.status();
    let set_cookie = response
        .headers()
        .get(header::SET_COOKIE)
        .map(|v| v.to_str().unwrap().to_owned());
    let bytes = response.into_body().collect().await.unwrap().to_bytes();
    Reply {
        status,
        set_cookie,
        json: serde_json::from_slice(&bytes).unwrap_or(Value::Null),
    }
}

struct Member {
    id: String,
    name: String,
    token: String,
}

async fn register(app: &Router, prefix: &str) -> Member {
    let name = format!("{prefix}_{}", unique_suffix());
    let reply = send(
        app,
        Method::POST,
        "/api/v0/auth/register",
        Some(json!({ "username": name, "password": PASSWORD })),
        None,
    )
    .await;
    assert_eq!(reply.status, StatusCode::CREATED, "{}", reply.json);
    let token = reply
        .set_cookie
        .unwrap()
        .strip_prefix("oxistream_session=")
        .unwrap()
        .split(';')
        .next()
        .unwrap()
        .to_owned();
    Member {
        id: reply.json["id"].as_str().unwrap().to_owned(),
        name,
        token,
    }
}

async fn login(app: &Router, name: &str) -> Reply {
    send(
        app,
        Method::POST,
        "/api/v0/auth/login",
        Some(json!({ "username": name, "password": PASSWORD })),
        None,
    )
    .await
}

async fn me(app: &Router, member: &Member) -> StatusCode {
    send(
        app,
        Method::GET,
        "/api/v0/auth/me",
        None,
        Some(&member.token),
    )
    .await
    .status
}

async fn patch(app: &Router, admin: &Member, id: &str, body: Value) -> Reply {
    send(
        app,
        Method::PATCH,
        &format!("/api/v0/admin/users/{id}"),
        Some(body),
        Some(&admin.token),
    )
    .await
}

async fn delete(app: &Router, admin: &Member, id: &str) -> Reply {
    send(
        app,
        Method::DELETE,
        &format!("/api/v0/admin/users/{id}"),
        None,
        Some(&admin.token),
    )
    .await
}

/// A fresh instance whose first user is the admin.
async fn setup() -> (common::TestDb, Router, Member, Member) {
    let db = isolated_test_pool().await;
    let app = oxistream_server::app(state_with_pool(db.pool.clone()));
    let admin = register(&app, "admin").await;
    let bob = register(&app, "bob").await;
    (db, app, admin, bob)
}

fn users(pool: &PgPool) -> PgUserRepository {
    PgUserRepository::new(pool.clone())
}

#[tokio::test]
#[ignore = "requires Docker or OXISTREAM_TEST_DATABASE_URL"]
async fn admin_routes_require_an_admin() {
    let (_db, app, admin, bob) = setup().await;
    let target = format!("/api/v0/admin/users/{}", admin.id);
    let routes = [
        (Method::GET, "/api/v0/admin/users", None),
        (Method::GET, "/api/v0/admin/status", None),
        (
            Method::PATCH,
            target.as_str(),
            Some(json!({ "role": "user" })),
        ),
        (Method::DELETE, target.as_str(), None),
    ];
    for (method, uri, body) in routes {
        let anonymous = send(&app, method.clone(), uri, body.clone(), None).await;
        assert_eq!(anonymous.status, StatusCode::UNAUTHORIZED, "{method} {uri}");
        let user = send(&app, method.clone(), uri, body, Some(&bob.token)).await;
        assert_eq!(user.status, StatusCode::FORBIDDEN, "{method} {uri}");
        assert_eq!(user.json["code"], "forbidden");
    }
    assert_eq!(me(&app, &admin).await, StatusCode::OK);
}

#[tokio::test]
#[ignore = "requires Docker or OXISTREAM_TEST_DATABASE_URL"]
async fn lists_users_with_search_and_pages() {
    let (db, app, admin, bob) = setup().await;
    users(&db.pool)
        .provision(
            &Identity {
                issuer: "https://idp.test".into(),
                subject: "carol".into(),
            },
            &|attempt| format!("carol_{attempt}"),
            Role::User,
        )
        .await
        .unwrap();

    let all = send(
        &app,
        Method::GET,
        "/api/v0/admin/users",
        None,
        Some(&admin.token),
    )
    .await;
    assert_eq!(all.status, StatusCode::OK, "{}", all.json);
    assert_eq!(all.json["total"], 3);
    let names: Vec<_> = all.json["users"]
        .as_array()
        .unwrap()
        .iter()
        .map(|u| u["username"].as_str().unwrap().to_owned())
        .collect();
    assert_eq!(names, ["carol_0", bob.name.as_str(), admin.name.as_str()]);
    let first = &all.json["users"][0];
    assert_eq!(first["auth_method"], "oidc");
    assert_eq!(first["last_seen_at"], Value::Null);
    let last = &all.json["users"][2];
    assert_eq!(last["role"], "admin");
    assert_eq!(last["auth_method"], "local");
    assert!(last["last_seen_at"].is_string());
    assert_eq!(last["subscriptions"], 0);

    let page = send(
        &app,
        Method::GET,
        "/api/v0/admin/users?limit=1&offset=1",
        None,
        Some(&admin.token),
    )
    .await;
    assert_eq!(page.json["total"], 3);
    assert_eq!(page.json["users"][0]["username"], bob.name);

    let search = send(
        &app,
        Method::GET,
        "/api/v0/admin/users?q=BOB",
        None,
        Some(&admin.token),
    )
    .await;
    assert_eq!(search.json["total"], 1);
    assert_eq!(search.json["users"][0]["id"], bob.id);
    let wildcard = send(
        &app,
        Method::GET,
        "/api/v0/admin/users?q=%25",
        None,
        Some(&admin.token),
    )
    .await;
    assert_eq!(wildcard.json["total"], 0, "LIKE wildcards are literal");

    let bad = send(
        &app,
        Method::GET,
        "/api/v0/admin/users?limit=0",
        None,
        Some(&admin.token),
    )
    .await;
    assert_eq!(bad.status, StatusCode::BAD_REQUEST);
}

#[tokio::test]
#[ignore = "requires Docker or OXISTREAM_TEST_DATABASE_URL"]
async fn changes_roles_and_access() {
    let (_db, app, admin, bob) = setup().await;

    let promoted = patch(&app, &admin, &bob.id, json!({ "role": "admin" })).await;
    assert_eq!(promoted.status, StatusCode::NO_CONTENT, "{}", promoted.json);
    let listed = send(
        &app,
        Method::GET,
        "/api/v0/admin/users",
        None,
        Some(&bob.token),
    )
    .await;
    assert_eq!(listed.status, StatusCode::OK, "bob is an admin now");
    assert_eq!(
        patch(&app, &admin, &bob.id, json!({ "role": "user" }))
            .await
            .status,
        StatusCode::NO_CONTENT
    );

    let disabled = patch(&app, &admin, &bob.id, json!({ "disabled": true })).await;
    assert_eq!(disabled.status, StatusCode::NO_CONTENT);
    assert_eq!(me(&app, &bob).await, StatusCode::UNAUTHORIZED);
    let login_reply = login(&app, &bob.name).await;
    assert_eq!(login_reply.status, StatusCode::FORBIDDEN);
    assert_eq!(login_reply.json["code"], "account_disabled");

    assert_eq!(
        patch(&app, &admin, &bob.id, json!({ "disabled": false }))
            .await
            .status,
        StatusCode::NO_CONTENT
    );
    assert_eq!(
        me(&app, &bob).await,
        StatusCode::UNAUTHORIZED,
        "sessions stay ended"
    );
    assert_eq!(login(&app, &bob.name).await.status, StatusCode::OK);

    let unknown = patch(
        &app,
        &admin,
        &Uuid::nil().to_string(),
        json!({ "role": "admin" }),
    )
    .await;
    assert_eq!(unknown.status, StatusCode::NOT_FOUND);
    assert_eq!(
        patch(&app, &admin, "not-a-uuid", json!({})).await.status,
        StatusCode::NOT_FOUND
    );
    assert_eq!(
        patch(&app, &admin, &bob.id, json!({ "role": "owner" }))
            .await
            .status,
        StatusCode::UNPROCESSABLE_ENTITY
    );
}

#[tokio::test]
#[ignore = "requires Docker or OXISTREAM_TEST_DATABASE_URL"]
async fn admins_cannot_change_their_own_account() {
    let (_db, app, admin, _bob) = setup().await;
    for body in [
        json!({ "role": "user" }),
        json!({ "disabled": true }),
        json!({}),
    ] {
        let reply = patch(&app, &admin, &admin.id, body).await;
        assert_eq!(reply.status, StatusCode::CONFLICT);
        assert_eq!(reply.json["code"], "own_account");
    }
    let reply = delete(&app, &admin, &admin.id).await;
    assert_eq!(reply.status, StatusCode::CONFLICT);
    assert_eq!(reply.json["code"], "own_account");
    assert_eq!(me(&app, &admin).await, StatusCode::OK);
}

#[tokio::test]
#[ignore = "requires Docker or OXISTREAM_TEST_DATABASE_URL"]
async fn deletes_users_and_their_sessions() {
    let (_db, app, admin, bob) = setup().await;
    assert_eq!(
        delete(&app, &admin, &bob.id).await.status,
        StatusCode::NO_CONTENT
    );
    assert_eq!(me(&app, &bob).await, StatusCode::UNAUTHORIZED);
    assert_eq!(
        login(&app, &bob.name).await.status,
        StatusCode::UNAUTHORIZED
    );
    assert_eq!(
        delete(&app, &admin, &bob.id).await.status,
        StatusCode::NOT_FOUND
    );
    assert_eq!(
        delete(&app, &admin, "nope").await.status,
        StatusCode::NOT_FOUND
    );
}

#[tokio::test]
#[ignore = "requires Docker or OXISTREAM_TEST_DATABASE_URL"]
async fn the_last_active_admin_is_kept() {
    let db = isolated_test_pool().await;
    let repo = users(&db.pool);
    let new = |name: &str| NewUser {
        username: format!("{name}_{}", unique_suffix()),
        password_hash: None,
        admin_if_none: true,
    };
    let admin = repo.create(&new("admin")).await.unwrap();
    let other = repo.create(&new("other")).await.unwrap();
    assert_eq!(admin.role, Role::Admin);

    assert_eq!(
        repo.admin_set_role(admin.id, Role::User).await.unwrap(),
        AdminUpdate::LastAdmin
    );
    assert_eq!(
        repo.admin_set_disabled(admin.id, true).await.unwrap(),
        AdminUpdate::LastAdmin
    );

    // A disabled second admin does not count.
    assert!(matches!(
        repo.admin_set_role(other.id, Role::Admin).await.unwrap(),
        AdminUpdate::Updated(_)
    ));
    assert!(matches!(
        repo.admin_set_disabled(other.id, true).await.unwrap(),
        AdminUpdate::Updated(_)
    ));
    assert_eq!(
        repo.admin_set_role(admin.id, Role::User).await.unwrap(),
        AdminUpdate::LastAdmin
    );

    assert!(matches!(
        repo.admin_set_disabled(other.id, false).await.unwrap(),
        AdminUpdate::Updated(_)
    ));
    let AdminUpdate::Updated(demoted) = repo.admin_set_role(admin.id, Role::User).await.unwrap()
    else {
        panic!("another active admin exists");
    };
    assert_eq!(demoted.role, Role::User);
    assert_eq!(
        repo.admin_set_role(Uuid::nil(), Role::User).await.unwrap(),
        AdminUpdate::NotFound
    );
}

#[tokio::test]
#[ignore = "requires Docker or OXISTREAM_TEST_DATABASE_URL"]
async fn reports_instance_status() {
    let (_db, app, admin, _bob) = setup().await;
    let reply = send(
        &app,
        Method::GET,
        "/api/v0/admin/status",
        None,
        Some(&admin.token),
    )
    .await;
    assert_eq!(reply.status, StatusCode::OK, "{}", reply.json);
    let status = reply.json;
    assert_eq!(status["version"], env!("CARGO_PKG_VERSION"));
    assert_eq!(status["database"]["reachable"], true);
    assert!(status["database"]["version"].is_string());
    assert_eq!(status["counts"]["users"], 2);
    assert_eq!(status["counts"]["admins"], 1);
    assert_eq!(status["counts"]["channels"], 0);
    // Test schemas share one database, so a parallel test may hold the advisory lock.
    assert!(status["feed"]["running"].is_boolean());
    assert_eq!(
        status["feed"]["interval_minutes"],
        Value::Null,
        "no worker in tests"
    );
    assert_eq!(status["trending_region"], "US");
    assert_eq!(status["auth"]["oidc_enabled"], false);
    assert!(status["player_cache_entries"].is_u64());
}

#[tokio::test]
#[ignore = "requires Docker or OXISTREAM_TEST_DATABASE_URL"]
async fn sees_the_feed_worker_lock() {
    let db = isolated_test_pool().await;
    let stats = PgStatsRepository::new(db.pool.clone());
    let lock = AdvisoryLock::try_acquire(&db.pool, LockKey::FeedWorker)
        .await
        .unwrap()
        .expect("lock is free");
    assert!(stats.lock_held(LockKey::FeedWorker).await.unwrap());
    assert!(!stats.lock_held(LockKey::HistoryRetention).await.unwrap());
    lock.release().await.unwrap();
    assert!(!stats.lock_held(LockKey::FeedWorker).await.unwrap());
}
