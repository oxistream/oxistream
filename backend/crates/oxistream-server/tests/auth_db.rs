//! Needs `OXISTREAM_TEST_DATABASE_URL` or a Docker daemon; run with `cargo test -- --include-ignored`.

mod common;

use axum::Router;
use axum::body::Body;
use axum::http::{Method, Request, StatusCode, header};
use common::{isolated_test_pool, state_with_auth, test_pool, unique_suffix};
use http_body_util::BodyExt;
use oxistream_db::{
    Identity, NewSession, NewUser, PgPool, PgSessionRepository, PgUserRepository, Role,
    SessionRepository, UserRepository,
};
use oxistream_server::auth::cookie;
use oxistream_server::config::AuthConfig;
use serde_json::{Value, json};
use time::OffsetDateTime;
use tower::ServiceExt;

const ORIGIN: &str = "http://tube.test";
const HOST: &str = "tube.test";
const PASSWORD: &str = "correct horse battery";

struct Reply {
    status: StatusCode,
    set_cookie: Option<String>,
    json: Value,
}

impl Reply {
    /// The session token from `Set-Cookie`, if a session was started.
    fn token(&self) -> Option<String> {
        let value = self.set_cookie.as_deref()?;
        let token = value
            .strip_prefix("oxistream_session=")?
            .split(';')
            .next()?;
        (!token.is_empty()).then(|| token.to_owned())
    }
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
        .header(header::HOST, HOST)
        .header(header::ORIGIN, ORIGIN)
        .header(header::USER_AGENT, "auth-test");
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
    let json = serde_json::from_slice(&bytes).unwrap_or(Value::Null);
    Reply {
        status,
        set_cookie,
        json,
    }
}

fn credentials(username: &str, password: &str) -> Value {
    json!({ "username": username, "password": password })
}

fn app(pool: &PgPool, config: AuthConfig) -> Router {
    oxistream_server::app(state_with_auth(pool.clone(), config))
}

fn username() -> String {
    format!("u_{}", unique_suffix())
}

async fn register(app: &Router, username: &str) -> Reply {
    let reply = send(
        app,
        Method::POST,
        "/api/v0/auth/register",
        Some(credentials(username, PASSWORD)),
        None,
    )
    .await;
    assert_eq!(reply.status, StatusCode::CREATED, "{}", reply.json);
    reply
}

#[tokio::test]
#[ignore = "requires Docker or OXISTREAM_TEST_DATABASE_URL"]
async fn register_login_me_logout() {
    let db = test_pool().await;
    let app = app(&db.pool, AuthConfig::default());
    let name = username();

    let registered = register(&app, &name).await;
    assert_eq!(registered.json["username"], name);
    let cookie = registered.set_cookie.clone().unwrap();
    assert!(cookie.contains("HttpOnly"), "{cookie}");
    assert!(cookie.contains("SameSite=Lax"), "{cookie}");
    assert!(cookie.contains("Secure"), "{cookie}");
    assert!(
        cookie.contains(&format!("Max-Age={}", 30 * 24 * 3600)),
        "{cookie}"
    );
    let token = registered.token().unwrap();

    let me = send(&app, Method::GET, "/api/v0/auth/me", None, Some(&token)).await;
    assert_eq!(me.status, StatusCode::OK);
    assert_eq!(me.json["id"], registered.json["id"]);
    assert!(me.set_cookie.is_none(), "fresh sessions are not renewed");

    let anonymous = send(&app, Method::GET, "/api/v0/auth/me", None, None).await;
    assert_eq!(anonymous.status, StatusCode::UNAUTHORIZED);
    assert_eq!(anonymous.json["code"], "unauthorized");

    let login = send(
        &app,
        Method::POST,
        "/api/v0/auth/login",
        Some(credentials(&name.to_uppercase(), PASSWORD)),
        Some(&token),
    )
    .await;
    assert_eq!(login.status, StatusCode::OK, "{}", login.json);
    let new_token = login.token().unwrap();
    assert_ne!(new_token, token);
    let old = send(&app, Method::GET, "/api/v0/auth/me", None, Some(&token)).await;
    assert_eq!(
        old.status,
        StatusCode::UNAUTHORIZED,
        "login replaces the old session"
    );

    let logout = send(
        &app,
        Method::POST,
        "/api/v0/auth/logout",
        None,
        Some(&new_token),
    )
    .await;
    assert_eq!(logout.status, StatusCode::OK);
    assert_eq!(logout.json, json!({ "end_session_url": null }));
    assert!(logout.set_cookie.unwrap().contains("Max-Age=0"));
    let after = send(&app, Method::GET, "/api/v0/auth/me", None, Some(&new_token)).await;
    assert_eq!(after.status, StatusCode::UNAUTHORIZED);
    assert!(
        after.set_cookie.unwrap().contains("Max-Age=0"),
        "unknown session cookies are cleared"
    );
}

#[tokio::test]
#[ignore = "requires Docker or OXISTREAM_TEST_DATABASE_URL"]
async fn bad_credentials_are_indistinguishable() {
    let db = test_pool().await;
    let app = app(&db.pool, AuthConfig::default());
    let name = username();
    register(&app, &name).await;

    let wrong_password = send(
        &app,
        Method::POST,
        "/api/v0/auth/login",
        Some(credentials(&name, "wrong password!")),
        None,
    )
    .await;
    let unknown_user = send(
        &app,
        Method::POST,
        "/api/v0/auth/login",
        Some(credentials(&username(), PASSWORD)),
        None,
    )
    .await;
    for reply in [&wrong_password, &unknown_user] {
        assert_eq!(reply.status, StatusCode::UNAUTHORIZED);
        assert_eq!(reply.json["code"], "invalid_credentials");
        assert!(reply.set_cookie.is_none());
    }
    assert_eq!(wrong_password.json, unknown_user.json);
}

#[tokio::test]
#[ignore = "requires Docker or OXISTREAM_TEST_DATABASE_URL"]
async fn first_user_becomes_admin_exactly_once() {
    let db = isolated_test_pool().await;
    let repo = PgUserRepository::new(db.pool.clone());
    let users = futures_util::future::join_all((0..8).map(|i| {
        let repo = repo.clone();
        async move {
            repo.create(&NewUser {
                username: format!("racer{i}"),
                password_hash: None,
            })
            .await
            .unwrap()
        }
    }))
    .await;
    let admins = users
        .iter()
        .filter(|u| u.role == oxistream_db::Role::Admin)
        .count();
    assert_eq!(admins, 1);

    let app = app(&db.pool, AuthConfig::default());
    let later = register(&app, &username()).await;
    assert_eq!(later.json["role"], "user");
}

#[tokio::test]
#[ignore = "requires Docker or OXISTREAM_TEST_DATABASE_URL"]
async fn provisioning_uses_the_given_role_and_sync_demotes_the_last_admin() {
    let db = isolated_test_pool().await;
    let repo = PgUserRepository::new(db.pool.clone());
    let identity = |subject: &str| Identity {
        issuer: "https://idp.test".into(),
        subject: subject.into(),
    };
    let first = repo
        .provision(&identity("first"), &|i| format!("first{i}"), Role::User)
        .await
        .unwrap();
    assert_eq!(first.role, Role::User, "no first-user rule for OIDC");
    let admin = repo
        .provision(&identity("admin"), &|i| format!("admin{i}"), Role::Admin)
        .await
        .unwrap();
    assert_eq!(admin.role, Role::Admin);
    let again = repo
        .provision(&identity("admin"), &|i| format!("other{i}"), Role::User)
        .await
        .unwrap();
    assert_eq!(again, admin, "already provisioned");

    let demoted = repo.sync_role(admin.id, Role::User).await.unwrap().unwrap();
    assert_eq!(demoted.role, Role::User, "the last admin is demoted");
    let unchanged = repo.sync_role(admin.id, Role::User).await.unwrap().unwrap();
    assert_eq!(unchanged.updated_at, demoted.updated_at);
    assert_eq!(
        repo.sync_role(oxistream_db::Uuid::nil(), Role::Admin)
            .await
            .unwrap(),
        None
    );
}

#[tokio::test]
#[ignore = "requires Docker or OXISTREAM_TEST_DATABASE_URL"]
async fn first_registered_account_is_admin() {
    let db = isolated_test_pool().await;
    let app = app(&db.pool, AuthConfig::default());
    assert_eq!(register(&app, "first").await.json["role"], "admin");
    assert_eq!(register(&app, "second").await.json["role"], "user");
}

#[tokio::test]
#[ignore = "requires Docker or OXISTREAM_TEST_DATABASE_URL"]
async fn usernames_are_unique_ignoring_case() {
    let db = test_pool().await;
    let app = app(&db.pool, AuthConfig::default());
    let name = format!("Mara_{}", unique_suffix());
    register(&app, &name).await;

    let duplicate = send(
        &app,
        Method::POST,
        "/api/v0/auth/register",
        Some(credentials(&name.to_lowercase(), PASSWORD)),
        None,
    )
    .await;
    assert_eq!(duplicate.status, StatusCode::CONFLICT);
    assert_eq!(duplicate.json["code"], "validation_failed");
    assert_eq!(duplicate.json["fields"]["username"], "taken");
}

#[tokio::test]
#[ignore = "requires Docker or OXISTREAM_TEST_DATABASE_URL"]
async fn registration_validates_input() {
    let db = test_pool().await;
    let app = app(&db.pool, AuthConfig::default());
    let reply = send(
        &app,
        Method::POST,
        "/api/v0/auth/register",
        Some(credentials("a b", "short")),
        None,
    )
    .await;
    assert_eq!(reply.status, StatusCode::UNPROCESSABLE_ENTITY);
    assert_eq!(
        reply.json["fields"],
        json!({ "username": "format", "password": "length" })
    );

    let malformed = send(
        &app,
        Method::POST,
        "/api/v0/auth/register",
        Some(json!({ "username": "x" })),
        None,
    )
    .await;
    assert_eq!(malformed.status, StatusCode::UNPROCESSABLE_ENTITY);
    assert_eq!(malformed.json["code"], "bad_request");
}

#[tokio::test]
#[ignore = "requires Docker or OXISTREAM_TEST_DATABASE_URL"]
async fn disabled_users_cannot_sign_in_or_use_sessions() {
    let db = test_pool().await;
    let app = app(&db.pool, AuthConfig::default());
    let name = username();
    let registered = register(&app, &name).await;
    let token = registered.token().unwrap();
    let id = registered.json["id"].as_str().unwrap().parse().unwrap();

    assert!(
        PgUserRepository::new(db.pool.clone())
            .set_disabled(id, true)
            .await
            .unwrap()
    );
    let me = send(&app, Method::GET, "/api/v0/auth/me", None, Some(&token)).await;
    assert_eq!(me.status, StatusCode::UNAUTHORIZED);

    let login = send(
        &app,
        Method::POST,
        "/api/v0/auth/login",
        Some(credentials(&name, PASSWORD)),
        None,
    )
    .await;
    assert_eq!(login.status, StatusCode::FORBIDDEN);
    assert_eq!(login.json["code"], "account_disabled");
    assert!(login.set_cookie.is_none());

    let wrong = send(
        &app,
        Method::POST,
        "/api/v0/auth/login",
        Some(credentials(&name, "wrong password!")),
        None,
    )
    .await;
    assert_eq!(
        wrong.json["code"], "invalid_credentials",
        "disabled state is only revealed with the right password"
    );
}

#[tokio::test]
#[ignore = "requires Docker or OXISTREAM_TEST_DATABASE_URL"]
async fn expired_sessions_are_rejected_and_cleaned_up() {
    let db = test_pool().await;
    let app = app(&db.pool, AuthConfig::default());
    let user = PgUserRepository::new(db.pool.clone())
        .create(&NewUser {
            username: username(),
            password_hash: None,
        })
        .await
        .unwrap();
    let sessions = PgSessionRepository::new(db.pool.clone());
    let (token, token_hash) = cookie::new_token();
    sessions
        .create(&NewSession {
            token_hash,
            user_id: user.id,
            expires_at: OffsetDateTime::now_utc() - time::Duration::minutes(1),
            user_agent: None,
            ip: Some("192.0.2.1".parse().unwrap()),
            oidc_sid: None,
            oidc_id_token: None,
        })
        .await
        .unwrap();

    let me = send(&app, Method::GET, "/api/v0/auth/me", None, Some(&token)).await;
    assert_eq!(me.status, StatusCode::UNAUTHORIZED);

    assert!(sessions.delete_expired().await.unwrap() >= 1);
    let left: i64 = sqlx::query_scalar("SELECT count(*) FROM sessions WHERE id = $1")
        .bind(token_hash.as_slice())
        .fetch_one(&db.pool)
        .await
        .unwrap();
    assert_eq!(left, 0);
}

#[tokio::test]
#[ignore = "requires Docker or OXISTREAM_TEST_DATABASE_URL"]
async fn sessions_slide_when_used() {
    let db = test_pool().await;
    let app = app(&db.pool, AuthConfig::default());
    let registered = register(&app, &username()).await;
    let token = registered.token().unwrap();
    let hash = cookie::hash_token(&token).unwrap();

    sqlx::query(
        "UPDATE sessions SET last_seen_at = now() - interval '10 minutes', \
            expires_at = now() + interval '1 day' WHERE id = $1",
    )
    .bind(hash.as_slice())
    .execute(&db.pool)
    .await
    .unwrap();

    let me = send(&app, Method::GET, "/api/v0/auth/me", None, Some(&token)).await;
    assert_eq!(me.status, StatusCode::OK);
    let renewed = me.set_cookie.expect("renewed cookie");
    assert!(
        renewed.contains(&format!("oxistream_session={token};")),
        "{renewed}"
    );
    assert!(renewed.contains(&format!("Max-Age={}", 30 * 24 * 3600)));

    let expires_at: OffsetDateTime =
        sqlx::query_scalar("SELECT expires_at FROM sessions WHERE id = $1")
            .bind(hash.as_slice())
            .fetch_one(&db.pool)
            .await
            .unwrap();
    assert!(expires_at > OffsetDateTime::now_utc() + time::Duration::days(29));
}

#[tokio::test]
#[ignore = "requires Docker or OXISTREAM_TEST_DATABASE_URL"]
async fn require_login_rejects_anonymous_requests() {
    let db = test_pool().await;
    let open = app(&db.pool, AuthConfig::default());
    let token = register(&open, &username()).await.token().unwrap();
    let app = app(
        &db.pool,
        AuthConfig {
            require_login: true,
            ..AuthConfig::default()
        },
    );

    for uri in [
        "/api/v0/search/suggestions?q=",
        "/proxy/image/aHR0cHM6Ly9pLnl0aW1nLmNvbS92aS94L2hxLmpwZw?exp=1&sig=x",
    ] {
        let reply = send(&app, Method::GET, uri, None, None).await;
        assert_eq!(reply.status, StatusCode::UNAUTHORIZED, "{uri}");
        assert_eq!(reply.json["code"], "unauthorized");
    }
    let signed_in = send(
        &app,
        Method::GET,
        "/api/v0/search/suggestions?q=",
        None,
        Some(&token),
    )
    .await;
    assert_eq!(signed_in.status, StatusCode::OK);

    let instance = send(&app, Method::GET, "/api/v0/instance", None, None).await;
    assert_eq!(instance.status, StatusCode::OK);
    assert_eq!(instance.json["auth"]["require_login"], true);
    let login = send(
        &app,
        Method::POST,
        "/api/v0/auth/login",
        Some(credentials("nobody_here", PASSWORD)),
        None,
    )
    .await;
    assert_eq!(login.status, StatusCode::UNAUTHORIZED);
    assert_eq!(login.json["code"], "invalid_credentials");
}

#[tokio::test]
#[ignore = "requires Docker or OXISTREAM_TEST_DATABASE_URL"]
async fn disabled_registration_and_local_auth() {
    let db = test_pool().await;
    let closed = app(
        &db.pool,
        AuthConfig {
            registration_enabled: false,
            ..AuthConfig::default()
        },
    );
    let reply = send(
        &closed,
        Method::POST,
        "/api/v0/auth/register",
        Some(credentials(&username(), PASSWORD)),
        None,
    )
    .await;
    assert_eq!(reply.status, StatusCode::FORBIDDEN);
    assert_eq!(reply.json["code"], "registration_disabled");

    let no_local = app(
        &db.pool,
        AuthConfig {
            local_enabled: false,
            ..AuthConfig::default()
        },
    );
    let reply = send(
        &no_local,
        Method::POST,
        "/api/v0/auth/login",
        Some(credentials(&username(), PASSWORD)),
        None,
    )
    .await;
    assert_eq!(reply.status, StatusCode::FORBIDDEN);
    assert_eq!(reply.json["code"], "local_auth_disabled");
    let instance = send(&no_local, Method::GET, "/api/v0/instance", None, None).await;
    assert_eq!(
        instance.json["auth"],
        json!({
            "local_enabled": false,
            "registration_enabled": false,
            "require_login": false,
            "oidc_enabled": false,
            "oidc_name": null,
        })
    );
}
