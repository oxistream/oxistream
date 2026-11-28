//! Single sign-on against an in-process provider. Tests marked `#[ignore]` need
//! `OXISTREAM_TEST_DATABASE_URL` or a Docker daemon; run with `cargo test -- --include-ignored`.

mod common;

use std::collections::HashMap;

use axum::Router;
use axum::body::Body;
use axum::http::{Method, Request, StatusCode, header};
use common::idp::{CLIENT_ID, Idp};
use common::{TEST_PROXY_SECRET, isolated_test_pool, state_with_auth, test_pool, unique_suffix};
use http_body_util::BodyExt;
use oxistream_db::{
    AdminUpdate, BackchannelLogout, LogoutOutcome, PgPool, PgSessionRepository, PgUserRepository,
    Role, SessionRepository, UserRepository,
};
use oxistream_server::AppState;
use oxistream_server::auth::cookie;
use oxistream_server::config::{AuthConfig, OidcConfig};
use reqwest::Url;
use serde_json::{Value, json};
use tower::ServiceExt;

const PUBLIC: &str = "http://tube.test";

struct Reply {
    status: StatusCode,
    location: Option<String>,
    cookies: Vec<String>,
    json: Value,
}

impl Reply {
    fn cookie(&self, name: &str) -> Option<String> {
        self.cookies.iter().find_map(|c| {
            let value = c.strip_prefix(&format!("{name}="))?.split(';').next()?;
            (!value.is_empty()).then(|| value.to_owned())
        })
    }

    fn clears(&self, name: &str) -> bool {
        self.cookies
            .iter()
            .any(|c| c.starts_with(&format!("{name}=;")) && c.contains("Max-Age=0"))
    }

    fn query(&self) -> HashMap<String, String> {
        let location = self.location.as_deref().expect("redirect");
        Url::parse(PUBLIC)
            .unwrap()
            .join(location)
            .unwrap()
            .query_pairs()
            .into_owned()
            .collect()
    }
}

async fn send(app: &Router, method: Method, uri: &str, cookies: &[(&str, &str)]) -> Reply {
    let mut req = Request::builder()
        .method(method)
        .uri(uri)
        .header(header::HOST, "tube.test")
        .header(header::ORIGIN, PUBLIC);
    if !cookies.is_empty() {
        let value = cookies
            .iter()
            .map(|(k, v)| format!("{k}={v}"))
            .collect::<Vec<_>>()
            .join("; ");
        req = req.header(header::COOKIE, value);
    }
    let response = app
        .clone()
        .oneshot(req.body(Body::empty()).unwrap())
        .await
        .unwrap();
    let status = response.status();
    let location = response
        .headers()
        .get(header::LOCATION)
        .map(|v| v.to_str().unwrap().to_owned());
    let cookies = response
        .headers()
        .get_all(header::SET_COOKIE)
        .iter()
        .map(|v| v.to_str().unwrap().to_owned())
        .collect();
    let bytes = response.into_body().collect().await.unwrap().to_bytes();
    Reply {
        status,
        location,
        cookies,
        json: serde_json::from_slice(&bytes).unwrap_or(Value::Null),
    }
}

fn oidc_config(idp: &Idp) -> OidcConfig {
    OidcConfig::new(&idp.issuer, CLIENT_ID, PUBLIC, TEST_PROXY_SECRET)
}

async fn discovered(pool: PgPool, oidc: OidcConfig) -> AppState {
    discovered_with(pool, oidc, AuthConfig::default()).await
}

async fn discovered_with(pool: PgPool, oidc: OidcConfig, auth: AuthConfig) -> AppState {
    let state = state_with_auth(
        pool,
        AuthConfig {
            oidc: Some(oidc),
            ..auth
        },
    );
    state.auth.oidc().unwrap().discover().await.unwrap();
    state
}

/// The authorization request the browser is sent to.
struct Started {
    params: HashMap<String, String>,
    pending: String,
}

async fn start(app: &Router, redirect: &str) -> Started {
    let reply = send(
        app,
        Method::GET,
        &format!("/api/v0/auth/oidc/login?redirect={redirect}"),
        &[],
    )
    .await;
    assert_eq!(reply.status, StatusCode::FOUND);
    let pending = reply
        .cookie("oxistream_oidc")
        .expect("pending login cookie");
    let params = reply.query();
    Started { params, pending }
}

async fn callback(app: &Router, query: &str, pending: Option<&str>) -> Reply {
    let cookies: Vec<(&str, &str)> = pending.map(|p| ("oxistream_oidc", p)).into_iter().collect();
    send(
        app,
        Method::GET,
        &format!("/api/v0/auth/oidc/callback?{query}"),
        &cookies,
    )
    .await
}

/// Runs the whole flow for a user with `claims` (which must contain `sub`).
async fn sign_in(app: &Router, idp: &Idp, claims: Value) -> Reply {
    let started = start(app, "%2Ffeed%2Fhistory").await;
    let mut claims = claims;
    claims["nonce"] = json!(started.params["nonce"]);
    let code = idp.grant(&started.params["code_challenge"], claims);
    callback(
        app,
        &format!("code={code}&state={}", started.params["state"]),
        Some(&started.pending),
    )
    .await
}

async fn me(app: &Router, reply: &Reply) -> Value {
    let token = reply.cookie("oxistream_session").expect("session cookie");
    let me = send(
        app,
        Method::GET,
        "/api/v0/auth/me",
        &[("oxistream_session", &token)],
    )
    .await;
    assert_eq!(me.status, StatusCode::OK, "{}", me.json);
    me.json
}

fn sub() -> String {
    format!("sub-{}", unique_suffix())
}

#[tokio::test]
async fn login_redirects_to_the_provider_with_pkce_state_and_nonce() {
    let idp = Idp::start().await;
    let state = discovered(common::unreachable_pool(), oidc_config(&idp)).await;
    let app = oxistream_server::app(state);
    let reply = send(
        &app,
        Method::GET,
        "/api/v0/auth/oidc/login?redirect=%2Fwatch%3Fv%3Dx",
        &[],
    )
    .await;
    assert_eq!(reply.status, StatusCode::FOUND);
    let location = reply.location.clone().unwrap();
    assert!(
        location.starts_with(&format!("{}/authorize?", idp.issuer)),
        "{location}"
    );
    let params = reply.query();
    assert_eq!(params["response_type"], "code");
    assert_eq!(params["client_id"], CLIENT_ID);
    assert_eq!(
        params["redirect_uri"],
        "http://tube.test/api/v0/auth/oidc/callback"
    );
    assert_eq!(params["scope"], "openid profile email");
    assert_eq!(params["code_challenge_method"], "S256");
    assert!(params["state"].len() >= 16 && params["nonce"].len() >= 16);
    let cookie = reply
        .cookies
        .iter()
        .find(|c| c.starts_with("oxistream_oidc="))
        .unwrap();
    assert!(cookie.contains("HttpOnly"), "{cookie}");
    assert!(cookie.contains("Path=/api/v0/auth/oidc"), "{cookie}");
    assert!(cookie.contains("Max-Age=600"), "{cookie}");
}

#[tokio::test]
async fn sign_in_fails_cleanly_without_discovery_or_configuration() {
    let idp = Idp::start().await;
    let undiscovered = oxistream_server::app(state_with_auth(
        common::unreachable_pool(),
        AuthConfig {
            oidc: Some(oidc_config(&idp)),
            ..AuthConfig::default()
        },
    ));
    let reply = send(
        &undiscovered,
        Method::GET,
        "/api/v0/auth/oidc/login?redirect=%2Ffeed",
        &[],
    )
    .await;
    assert_eq!(reply.status, StatusCode::FOUND);
    assert_eq!(
        reply.location.as_deref(),
        Some("/login?error=oidc_unavailable&redirect=%2Ffeed")
    );
    let (status, body) = common::get(undiscovered.clone(), "/readyz").await;
    assert_eq!(status, StatusCode::SERVICE_UNAVAILABLE);
    assert_eq!(body, "OIDC provider not discovered");

    let disabled = common::app_without_db();
    let reply = send(&disabled, Method::GET, "/api/v0/auth/oidc/login", &[]).await;
    assert_eq!(
        reply.location.as_deref(),
        Some("/login?error=oidc_disabled")
    );
    let reply = callback(&disabled, "code=x&state=y", None).await;
    assert_eq!(
        reply.location.as_deref(),
        Some("/login?error=oidc_disabled")
    );
}

#[tokio::test]
async fn instance_advertises_single_sign_on() {
    let idp = Idp::start().await;
    let mut config = oidc_config(&idp);
    config.name = "Authentik".into();
    let app = oxistream_server::app(discovered(common::unreachable_pool(), config).await);
    let reply = send(&app, Method::GET, "/api/v0/instance", &[]).await;
    assert_eq!(reply.json["auth"]["oidc_enabled"], true);
    assert_eq!(reply.json["auth"]["oidc_name"], "Authentik");
}

#[tokio::test]
#[ignore = "requires Docker or OXISTREAM_TEST_DATABASE_URL"]
async fn login_callback_creates_a_session() {
    let db = test_pool().await;
    let idp = Idp::start().await;
    let state = discovered(db.pool.clone(), oidc_config(&idp)).await;
    let app = oxistream_server::app(state);
    let subject = sub();
    let name = format!("mara_{}", unique_suffix());

    let reply = sign_in(
        &app,
        &idp,
        json!({ "sub": subject, "preferred_username": name, "sid": "idp-session-1" }),
    )
    .await;
    assert_eq!(reply.status, StatusCode::FOUND, "{:?}", reply.location);
    assert_eq!(reply.location.as_deref(), Some("/feed/history"));
    assert!(reply.clears("oxistream_oidc"));
    let account = me(&app, &reply).await;
    assert_eq!(account["username"], name);

    let token = reply.cookie("oxistream_session").unwrap();
    let hash = cookie::hash_token(&token).unwrap();
    let (sid, id_token): (Option<String>, Option<String>) =
        sqlx::query_as("SELECT oidc_sid, oidc_id_token FROM sessions WHERE id = $1")
            .bind(hash.as_slice())
            .fetch_one(&db.pool)
            .await
            .unwrap();
    assert_eq!(sid.as_deref(), Some("idp-session-1"));
    let id_token = id_token.expect("stored ID token");
    let password: Option<String> =
        sqlx::query_scalar("SELECT password_hash FROM users WHERE username = $1")
            .bind(&name)
            .fetch_one(&db.pool)
            .await
            .unwrap();
    assert_eq!(password, None);

    let again = sign_in(
        &app,
        &idp,
        json!({ "sub": subject, "preferred_username": "x" }),
    )
    .await;
    assert_eq!(
        me(&app, &again).await["id"],
        account["id"],
        "same identity, same user"
    );

    let logout = send(
        &app,
        Method::POST,
        "/api/v0/auth/logout",
        &[("oxistream_session", &token)],
    )
    .await;
    assert_eq!(logout.status, StatusCode::OK);
    let url = Url::parse(logout.json["end_session_url"].as_str().unwrap()).unwrap();
    assert_eq!(url.path(), "/logout");
    let params: HashMap<_, _> = url.query_pairs().into_owned().collect();
    assert_eq!(params["id_token_hint"], id_token);
    assert_eq!(params["post_logout_redirect_uri"], "http://tube.test/");
    assert_eq!(params["client_id"], CLIENT_ID);
}

#[tokio::test]
#[ignore = "requires Docker or OXISTREAM_TEST_DATABASE_URL"]
async fn logout_is_local_without_end_session_endpoint() {
    let db = test_pool().await;
    let idp = Idp::start().await;
    idp.state.lock().unwrap().end_session = false;
    let app = oxistream_server::app(discovered(db.pool.clone(), oidc_config(&idp)).await);
    let reply = sign_in(&app, &idp, json!({ "sub": sub() })).await;
    let token = reply.cookie("oxistream_session").unwrap();
    let logout = send(
        &app,
        Method::POST,
        "/api/v0/auth/logout",
        &[("oxistream_session", &token)],
    )
    .await;
    assert_eq!(logout.json, json!({ "end_session_url": null }));
}

#[tokio::test]
#[ignore = "requires Docker or OXISTREAM_TEST_DATABASE_URL"]
async fn rejects_state_and_nonce_mismatches() {
    let db = test_pool().await;
    let idp = Idp::start().await;
    let app = oxistream_server::app(discovered(db.pool.clone(), oidc_config(&idp)).await);
    let failed = "/login?error=oidc_failed&redirect=%2Ffeed%2Fhistory";

    let started = start(&app, "%2Ffeed%2Fhistory").await;
    let code = idp.grant(
        &started.params["code_challenge"],
        json!({ "sub": sub(), "nonce": started.params["nonce"] }),
    );
    let reply = callback(
        &app,
        &format!("code={code}&state=forged"),
        Some(&started.pending),
    )
    .await;
    assert_eq!(reply.location.as_deref(), Some(failed));
    assert!(reply.cookie("oxistream_session").is_none());
    assert!(reply.clears("oxistream_oidc"));

    let reply = callback(
        &app,
        &format!("code={code}&state={}", started.params["state"]),
        None,
    )
    .await;
    assert_eq!(
        reply.location.as_deref(),
        Some("/login?error=oidc_failed"),
        "no login state"
    );
    let tampered = format!("{}x", started.pending);
    let reply = callback(
        &app,
        &format!("code={code}&state={}", started.params["state"]),
        Some(&tampered),
    )
    .await;
    assert_eq!(reply.location.as_deref(), Some("/login?error=oidc_failed"));

    let started = start(&app, "%2Ffeed%2Fhistory").await;
    let code = idp.grant(
        &started.params["code_challenge"],
        json!({ "sub": sub(), "nonce": "replayed-nonce" }),
    );
    let reply = callback(
        &app,
        &format!("code={code}&state={}", started.params["state"]),
        Some(&started.pending),
    )
    .await;
    assert_eq!(reply.location.as_deref(), Some(failed), "nonce mismatch");
    assert!(reply.cookie("oxistream_session").is_none());

    let started = start(&app, "%2Ffeed%2Fhistory").await;
    let code = idp.grant(
        "not-the-challenge",
        json!({ "sub": sub(), "nonce": started.params["nonce"] }),
    );
    let reply = callback(
        &app,
        &format!("code={code}&state={}", started.params["state"]),
        Some(&started.pending),
    )
    .await;
    assert_eq!(reply.location.as_deref(), Some(failed), "PKCE mismatch");

    let started = start(&app, "%2Ffeed%2Fhistory").await;
    let reply = callback(
        &app,
        &format!("error=access_denied&state={}", started.params["state"]),
        Some(&started.pending),
    )
    .await;
    assert_eq!(
        reply.location.as_deref(),
        Some("/login?error=oidc_denied&redirect=%2Ffeed%2Fhistory")
    );
}

#[tokio::test]
#[ignore = "requires Docker or OXISTREAM_TEST_DATABASE_URL"]
async fn rejects_tokens_for_other_audiences_or_issuers() {
    let db = test_pool().await;
    let idp = Idp::start().await;
    let app = oxistream_server::app(discovered(db.pool.clone(), oidc_config(&idp)).await);
    for claims in [
        json!({ "sub": sub(), "aud": "someone-else" }),
        json!({ "sub": sub(), "iss": "https://evil.example" }),
        json!({ "sub": sub(), "exp": 1 }),
    ] {
        let reply = sign_in(&app, &idp, claims.clone()).await;
        assert_eq!(
            reply.location.as_deref(),
            Some("/login?error=oidc_failed&redirect=%2Ffeed%2Fhistory"),
            "{claims}"
        );
    }
}

#[tokio::test]
#[ignore = "requires Docker or OXISTREAM_TEST_DATABASE_URL"]
async fn provisioning_without_admin_group_makes_users_and_ignores_closed_registration() {
    let db = isolated_test_pool().await;
    let idp = Idp::start().await;
    let state = discovered_with(
        db.pool.clone(),
        oidc_config(&idp),
        AuthConfig {
            registration_enabled: false,
            ..AuthConfig::default()
        },
    )
    .await;
    let app = oxistream_server::app(state);

    let first_sub = sub();
    let first = sign_in(
        &app,
        &idp,
        json!({ "sub": first_sub, "preferred_username": "mara@example.com", "groups": ["/admins"] }),
    )
    .await;
    let first = me(&app, &first).await;
    assert_eq!(first["role"], "user", "first user, no admin group");
    assert_eq!(first["username"], "mara");

    let second = sign_in(
        &app,
        &idp,
        json!({ "sub": sub(), "preferred_username": "Mara", "email": "mara@example.com" }),
    )
    .await;
    let second = me(&app, &second).await;
    assert_eq!(second["role"], "user");
    assert_eq!(
        second["username"], "Mara-2",
        "never linked by name or email"
    );
    assert_ne!(second["id"], first["id"]);

    let third = sign_in(&app, &idp, json!({ "sub": "248289761001", "name": "李" })).await;
    assert_eq!(me(&app, &third).await["username"], "248289761001");

    let id = first["id"].as_str().unwrap().parse().unwrap();
    let users = PgUserRepository::new(db.pool.clone());
    assert!(matches!(
        users.admin_set_role(id, Role::Admin).await.unwrap(),
        AdminUpdate::Updated(_)
    ));
    let reply = sign_in(&app, &idp, json!({ "sub": first_sub, "groups": [] })).await;
    assert_eq!(
        me(&app, &reply).await["role"],
        "admin",
        "a promotion in the admin area sticks"
    );
}

#[tokio::test]
#[ignore = "requires Docker or OXISTREAM_TEST_DATABASE_URL"]
async fn concurrent_first_sign_ins_provision_one_account() {
    let db = isolated_test_pool().await;
    let idp = Idp::start().await;
    let app = oxistream_server::app(discovered(db.pool.clone(), oidc_config(&idp)).await);
    let subject = sub();
    let replies = futures_util::future::join_all(
        (0..6).map(|_| sign_in(&app, &idp, json!({ "sub": subject }))),
    )
    .await;
    let mut ids = Vec::new();
    for reply in &replies {
        ids.push(me(&app, reply).await["id"].clone());
    }
    ids.dedup();
    assert_eq!(ids.len(), 1, "{ids:?}");
}

#[tokio::test]
#[ignore = "requires Docker or OXISTREAM_TEST_DATABASE_URL"]
async fn admin_group_is_synced_on_every_login_even_for_the_last_admin() {
    let db = isolated_test_pool().await;
    let idp = Idp::start().await;
    let mut config = oidc_config(&idp);
    config.admin_group = Some("/admins".into());
    let app = oxistream_server::app(discovered(db.pool.clone(), config).await);
    let (alice, bob, carol) = (sub(), sub(), sub());

    let reply = sign_in(&app, &idp, json!({ "sub": alice, "groups": ["/users"] })).await;
    assert_eq!(
        me(&app, &reply).await["role"],
        "user",
        "first user, no member"
    );

    let reply = sign_in(&app, &idp, json!({ "sub": bob, "groups": ["/admins"] })).await;
    assert_eq!(me(&app, &reply).await["role"], "admin", "group member");
    let reply = sign_in(&app, &idp, json!({ "sub": bob, "groups": [] })).await;
    assert_eq!(
        me(&app, &reply).await["role"],
        "user",
        "the last admin is demoted"
    );

    idp.state
        .lock()
        .unwrap()
        .userinfo
        .insert(carol.clone(), json!({ "sub": carol, "groups": "/admins" }));
    let reply = sign_in(&app, &idp, json!({ "sub": carol })).await;
    assert_eq!(
        me(&app, &reply).await["role"],
        "admin",
        "groups from userinfo"
    );

    let id = me(&app, &reply).await["id"]
        .as_str()
        .unwrap()
        .parse()
        .unwrap();
    let users = PgUserRepository::new(db.pool.clone());
    assert!(matches!(
        users.admin_set_role(id, Role::User).await.unwrap(),
        AdminUpdate::LastAdmin
    ));
    let reply = sign_in(&app, &idp, json!({ "sub": alice, "groups": "/admins" })).await;
    assert_eq!(me(&app, &reply).await["role"], "admin", "promoted");
    let reply = sign_in(&app, &idp, json!({ "sub": alice, "groups": ["/users"] })).await;
    assert_eq!(me(&app, &reply).await["role"], "user", "demoted");
}

#[tokio::test]
#[ignore = "requires Docker or OXISTREAM_TEST_DATABASE_URL"]
async fn disabled_users_cannot_sign_in() {
    let db = test_pool().await;
    let idp = Idp::start().await;
    let app = oxistream_server::app(discovered(db.pool.clone(), oidc_config(&idp)).await);
    let subject = sub();
    let reply = sign_in(&app, &idp, json!({ "sub": subject })).await;
    let id = me(&app, &reply).await["id"]
        .as_str()
        .unwrap()
        .parse()
        .unwrap();
    let users = PgUserRepository::new(db.pool.clone());
    assert!(users.set_disabled(id, true).await.unwrap());

    let reply = sign_in(&app, &idp, json!({ "sub": subject })).await;
    assert_eq!(
        reply.location.as_deref(),
        Some("/login?error=account_disabled&redirect=%2Ffeed%2Fhistory")
    );
    assert!(reply.cookie("oxistream_session").is_none());
}

#[tokio::test]
#[ignore = "requires Docker or OXISTREAM_TEST_DATABASE_URL"]
async fn rotated_signing_keys_are_refetched() {
    let db = test_pool().await;
    let idp = Idp::start().await;
    let app = oxistream_server::app(discovered(db.pool.clone(), oidc_config(&idp)).await);
    {
        let mut state = idp.state.lock().unwrap();
        state.keys.push(common::idp::SigningKey::new(1, "key-2"));
        state.active = 1;
    }
    let fetches = idp.state.lock().unwrap().jwks_fetches;
    let reply = sign_in(&app, &idp, json!({ "sub": sub() })).await;
    assert!(
        reply.cookie("oxistream_session").is_some(),
        "{:?}",
        reply.location
    );
    assert_eq!(idp.state.lock().unwrap().jwks_fetches, fetches + 1);
}

const LOGOUT_EVENT: &str = "http://schemas.openid.net/event/backchannel-logout";

/// A logout token for `claims`, completed with `iss`, `aud`, `iat`, `jti` and the event.
fn logout_token(idp: &Idp, claims: &Value) -> String {
    let state = idp.state.lock().unwrap();
    let mut full = json!({
        "iss": state.issuer,
        "aud": CLIENT_ID,
        "iat": common::idp::now(),
        "jti": format!("jti-{}", unique_suffix()),
        "events": { LOGOUT_EVENT: {} },
    });
    for (key, value) in claims.as_object().unwrap() {
        if value.is_null() {
            full.as_object_mut().unwrap().remove(key);
        } else {
            full[key] = value.clone();
        }
    }
    state.keys[state.active].sign(&json!({ "typ": "logout+jwt" }), &full)
}

/// Posts like a provider would: form-encoded, without `Origin` or cookies.
async fn backchannel(app: &Router, token: &str) -> (StatusCode, Option<String>, Value) {
    let body = format!("logout_token={token}");
    let response = app
        .clone()
        .oneshot(
            Request::post("/api/v0/auth/oidc/backchannel-logout")
                .header(header::HOST, "oxistream:8080")
                .header(header::CONTENT_TYPE, "application/x-www-form-urlencoded")
                .body(Body::from(body))
                .unwrap(),
        )
        .await
        .unwrap();
    let status = response.status();
    let cache = response
        .headers()
        .get(header::CACHE_CONTROL)
        .map(|v| v.to_str().unwrap().to_owned());
    let bytes = response.into_body().collect().await.unwrap().to_bytes();
    (
        status,
        cache,
        serde_json::from_slice(&bytes).unwrap_or(Value::Null),
    )
}

async fn signed_in(app: &Router, reply: &Reply) -> bool {
    let token = reply.cookie("oxistream_session").unwrap();
    send(
        app,
        Method::GET,
        "/api/v0/auth/me",
        &[("oxistream_session", &token)],
    )
    .await
    .status
        == StatusCode::OK
}

#[tokio::test]
async fn backchannel_logout_rejects_invalid_tokens() {
    let idp = Idp::start().await;
    let app =
        oxistream_server::app(discovered(common::unreachable_pool(), oidc_config(&idp)).await);
    for (claims, reason) in [
        (
            json!({ "sid": "s", "aud": "someone-else" }),
            "wrong audience",
        ),
        (
            json!({ "sid": "s", "iss": "https://evil.example" }),
            "wrong issuer",
        ),
        (json!({ "sid": "s", "nonce": "n" }), "nonce is not allowed"),
        (
            json!({ "sid": "s", "events": null }),
            "missing logout event",
        ),
        (json!({ "sid": "s", "iat": 1 }), "too old"),
        (json!({ "sid": "s", "exp": 1 }), "expired"),
        (json!({ "sid": "s", "jti": null }), "missing jti"),
        (json!({}), "neither sid nor sub"),
    ] {
        let (status, cache, body) = backchannel(&app, &logout_token(&idp, &claims)).await;
        assert_eq!(status, StatusCode::BAD_REQUEST, "{claims}");
        assert_eq!(cache.as_deref(), Some("no-store"));
        assert_eq!(body["error"], "invalid_request");
        assert_eq!(body["error_description"], reason, "{claims}");
    }
    let (status, _, body) = backchannel(&app, "not-a-jwt").await;
    assert_eq!(status, StatusCode::BAD_REQUEST);
    assert_eq!(body["error_description"], "malformed token");

    let (status, _, _) = backchannel(&common::app_without_db(), "x").await;
    assert_eq!(status, StatusCode::BAD_REQUEST, "without single sign-on");
    let undiscovered = oxistream_server::app(state_with_auth(
        common::unreachable_pool(),
        AuthConfig {
            oidc: Some(oidc_config(&idp)),
            ..AuthConfig::default()
        },
    ));
    let (status, _, body) =
        backchannel(&undiscovered, &logout_token(&idp, &json!({ "sid": "s" }))).await;
    assert_eq!(status, StatusCode::SERVICE_UNAVAILABLE);
    assert_eq!(body["error"], "temporarily_unavailable");
}

#[tokio::test]
async fn backchannel_logout_refetches_keys_once_for_unknown_key_ids() {
    let idp = Idp::start().await;
    let app =
        oxistream_server::app(discovered(common::unreachable_pool(), oidc_config(&idp)).await);
    let fetches = idp.state.lock().unwrap().jwks_fetches;
    // Signed with a key the provider does not publish.
    let rogue = common::idp::SigningKey::new(1, "rogue");
    let mut claims = json!({
        "iss": idp.issuer, "aud": CLIENT_ID, "iat": common::idp::now(), "jti": "j",
        "sid": "s", "events": { LOGOUT_EVENT: {} },
    });
    let token = rogue.sign(&json!({ "typ": "logout+jwt" }), &claims);
    let (status, _, body) = backchannel(&app, &token).await;
    assert_eq!(status, StatusCode::BAD_REQUEST);
    assert_eq!(body["error_description"], "no matching key");
    assert_eq!(idp.state.lock().unwrap().jwks_fetches, fetches + 1);

    // A known key ID with someone else's key fails without another fetch.
    claims["jti"] = json!("k");
    let forged = common::idp::SigningKey::new(1, "key-1").sign(&json!({}), &claims);
    let (status, _, body) = backchannel(&app, &forged).await;
    assert_eq!(status, StatusCode::BAD_REQUEST);
    assert_eq!(body["error_description"], "invalid signature");
    assert_eq!(idp.state.lock().unwrap().jwks_fetches, fetches + 1);

    let (status, _, body) = backchannel(
        &app,
        &common::idp::SigningKey::new(0, "key-1").sign(&json!({ "typ": "JWT" }), &claims),
    )
    .await;
    assert_eq!(status, StatusCode::BAD_REQUEST);
    assert_eq!(body["error_description"], "unexpected typ header");
}

#[tokio::test]
#[ignore = "requires Docker or OXISTREAM_TEST_DATABASE_URL"]
async fn backchannel_logout_ends_sessions_by_sid() {
    let db = test_pool().await;
    let idp = Idp::start().await;
    let app = oxistream_server::app(discovered(db.pool.clone(), oidc_config(&idp)).await);
    let subject = sub();
    let (sid_a, sid_b) = (format!("sid-a-{subject}"), format!("sid-b-{subject}"));
    let a = sign_in(&app, &idp, json!({ "sub": subject, "sid": sid_a })).await;
    let b = sign_in(&app, &idp, json!({ "sub": subject, "sid": sid_b })).await;

    let token = logout_token(&idp, &json!({ "sid": sid_a, "sub": subject }));
    let (status, cache, _) = backchannel(&app, &token).await;
    assert_eq!(status, StatusCode::OK);
    assert_eq!(cache.as_deref(), Some("no-store"));
    assert!(!signed_in(&app, &a).await, "session with that sid is gone");
    assert!(signed_in(&app, &b).await, "other provider sessions stay");

    let (status, _, body) = backchannel(&app, &token).await;
    assert_eq!(status, StatusCode::BAD_REQUEST, "replayed jti");
    assert_eq!(body["error_description"], "logout token was already used");

    let other = sign_in(&app, &idp, json!({ "sub": sub(), "sid": sid_b })).await;
    let token = logout_token(&idp, &json!({ "sid": sid_b, "sub": "someone-else" }));
    assert_eq!(backchannel(&app, &token).await.0, StatusCode::OK);
    assert!(
        signed_in(&app, &b).await,
        "sid only counts for its own subject"
    );
    assert!(signed_in(&app, &other).await);
}

#[tokio::test]
#[ignore = "requires Docker or OXISTREAM_TEST_DATABASE_URL"]
async fn backchannel_logout_ends_all_sessions_by_sub() {
    let db = test_pool().await;
    let idp = Idp::start().await;
    let app = oxistream_server::app(discovered(db.pool.clone(), oidc_config(&idp)).await);
    let subject = sub();
    let a = sign_in(&app, &idp, json!({ "sub": subject, "sid": "x1" })).await;
    let b = sign_in(&app, &idp, json!({ "sub": subject })).await;
    let bystander = sign_in(&app, &idp, json!({ "sub": sub() })).await;

    let (status, _, _) = backchannel(&app, &logout_token(&idp, &json!({ "sub": subject }))).await;
    assert_eq!(status, StatusCode::OK);
    assert!(!signed_in(&app, &a).await);
    assert!(!signed_in(&app, &b).await);
    assert!(signed_in(&app, &bystander).await);

    let unknown = logout_token(&idp, &json!({ "sub": "never-signed-in" }));
    assert_eq!(backchannel(&app, &unknown).await.0, StatusCode::OK);
}

#[tokio::test]
#[ignore = "requires Docker or OXISTREAM_TEST_DATABASE_URL"]
async fn backchannel_logout_accepts_rotated_keys() {
    let db = test_pool().await;
    let idp = Idp::start().await;
    let app = oxistream_server::app(discovered(db.pool.clone(), oidc_config(&idp)).await);
    let subject = sub();
    let session = sign_in(&app, &idp, json!({ "sub": subject })).await;
    {
        let mut state = idp.state.lock().unwrap();
        state.keys.push(common::idp::SigningKey::new(1, "key-2"));
        state.active = 1;
    }
    let fetches = idp.state.lock().unwrap().jwks_fetches;
    let (status, _, body) =
        backchannel(&app, &logout_token(&idp, &json!({ "sub": subject }))).await;
    assert_eq!(status, StatusCode::OK, "{body}");
    assert_eq!(idp.state.lock().unwrap().jwks_fetches, fetches + 1);
    assert!(!signed_in(&app, &session).await);
}

#[tokio::test]
#[ignore = "requires Docker or OXISTREAM_TEST_DATABASE_URL"]
async fn expired_logout_token_ids_are_cleaned_up() {
    let db = test_pool().await;
    let sessions = PgSessionRepository::new(db.pool.clone());
    let jti = format!("old-{}", unique_suffix());
    let logout = BackchannelLogout {
        jti: &jti,
        expires_at: time::OffsetDateTime::now_utc() - time::Duration::minutes(1),
        issuer: "https://idp.example",
        sid: Some("no-such-sid"),
        subject: None,
    };
    let end = || sessions.end_sessions_for_logout(&logout);
    assert_eq!(end().await.unwrap(), LogoutOutcome::Ended(0));
    assert_eq!(end().await.unwrap(), LogoutOutcome::Replayed);
    assert!(sessions.delete_expired_logout_jtis().await.unwrap() >= 1);
    assert_eq!(end().await.unwrap(), LogoutOutcome::Ended(0));
}

#[tokio::test]
#[ignore = "requires Docker or OXISTREAM_TEST_DATABASE_URL"]
async fn failed_backchannel_logout_can_be_retried_with_the_same_token() {
    let db = isolated_test_pool().await;
    let idp = Idp::start().await;
    let app = oxistream_server::app(discovered(db.pool.clone(), oidc_config(&idp)).await);
    let subject = sub();
    let session = sign_in(&app, &idp, json!({ "sub": subject, "sid": "sid-retry" })).await;
    let token = logout_token(&idp, &json!({ "sid": "sid-retry", "sub": subject }));

    for statement in [
        "CREATE FUNCTION fail_session_delete() RETURNS trigger LANGUAGE plpgsql \
         AS $$ BEGIN RAISE EXCEPTION 'injected failure'; END $$",
        "CREATE TRIGGER fail_session_delete BEFORE DELETE ON sessions \
         FOR EACH ROW EXECUTE FUNCTION fail_session_delete()",
    ] {
        sqlx::query(statement).execute(&db.pool).await.unwrap();
    }
    let (status, _, _) = backchannel(&app, &token).await;
    assert_eq!(status, StatusCode::INTERNAL_SERVER_ERROR);
    assert!(signed_in(&app, &session).await);
    sqlx::query("DROP TRIGGER fail_session_delete ON sessions")
        .execute(&db.pool)
        .await
        .unwrap();

    let (status, _, body) = backchannel(&app, &token).await;
    assert_eq!(status, StatusCode::OK, "jti was rolled back: {body}");
    assert!(!signed_in(&app, &session).await);
    let (status, _, body) = backchannel(&app, &token).await;
    assert_eq!(status, StatusCode::BAD_REQUEST);
    assert_eq!(body["error_description"], "logout token was already used");
}
