//! Needs `OXISTREAM_TEST_DATABASE_URL` or a Docker daemon; run with `cargo test -- --include-ignored`.

mod common;

use axum::Router;
use axum::body::Body;
use axum::http::{Method, Request, StatusCode, header};
use common::{isolated_test_pool, state_with_auth, unique_suffix};
use http_body_util::BodyExt;
use oxistream_db::{
    DeleteOutcome, HistoryRepository, Identity, NewSession, NewUser, PgHistoryRepository, PgPool,
    PgSessionRepository, PgUserRepository, PgUserSettingsRepository, ProgressUpdate, Role,
    SessionRepository, SettingsUpdate, UserRepository, UserSettings, UserSettingsRepository,
};
use oxistream_server::auth::cookie;
use oxistream_server::config::AuthConfig;
use serde_json::{Value, json};
use time::OffsetDateTime;
use tower::ServiceExt;

const PASSWORD: &str = "correct horse battery";
const NEW_PASSWORD: &str = "staple battery horse";

struct Reply {
    status: StatusCode,
    set_cookie: Option<String>,
    json: Value,
}

impl Reply {
    fn token(&self) -> String {
        let value = self.set_cookie.as_deref().unwrap();
        value
            .strip_prefix("oxistream_session=")
            .unwrap()
            .split(';')
            .next()
            .unwrap()
            .to_owned()
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

fn app(pool: &PgPool) -> Router {
    oxistream_server::app(state_with_auth(pool.clone(), AuthConfig::default()))
}

fn username() -> String {
    format!("u_{}", unique_suffix())
}

async fn register(app: &Router, username: &str) -> Reply {
    let reply = send(
        app,
        Method::POST,
        "/api/v0/auth/register",
        Some(json!({ "username": username, "password": PASSWORD })),
        None,
    )
    .await;
    assert_eq!(reply.status, StatusCode::CREATED, "{}", reply.json);
    reply
}

async fn login(app: &Router, username: &str, password: &str) -> Reply {
    send(
        app,
        Method::POST,
        "/api/v0/auth/login",
        Some(json!({ "username": username, "password": password })),
        None,
    )
    .await
}

#[tokio::test]
#[ignore = "requires Docker or OXISTREAM_TEST_DATABASE_URL"]
async fn settings_repository_updates_fields_independently() {
    let db = isolated_test_pool().await;
    let users = PgUserRepository::new(db.pool.clone());
    let settings = PgUserSettingsRepository::new(db.pool.clone());
    let user = users
        .create(&NewUser {
            username: username(),
            password_hash: None,
        })
        .await
        .unwrap();
    assert!(!user.has_password);

    assert_eq!(
        settings.get(user.id).await.unwrap(),
        UserSettings::default(),
        "defaults without a row"
    );
    let updated = settings
        .update(
            user.id,
            &SettingsUpdate {
                video_codecs: Some(vec!["vp9".into(), "h264".into()]),
                max_quality: Some(Some(1080)),
                home_sections: Some(Some(vec!["subscriptions".into()])),
                audio_language: Some(Some("pt-BR".into())),
                ..Default::default()
            },
        )
        .await
        .unwrap();
    assert_eq!(updated.video_codecs, ["vp9", "h264"]);
    assert_eq!(updated.max_quality, Some(1080));
    assert_eq!(updated.default_quality, None);
    assert_eq!(updated.home_sections, Some(vec!["subscriptions".into()]));
    assert_eq!(updated.audio_language.as_deref(), Some("pt-BR"));

    settings.set_history_paused(user.id, true).await.unwrap();
    let updated = settings
        .update(
            user.id,
            &SettingsUpdate {
                max_quality: Some(None),
                default_quality: Some(Some(720)),
                ..Default::default()
            },
        )
        .await
        .unwrap();
    assert!(updated.history_paused, "untouched by other updates");
    assert_eq!(updated.video_codecs, ["vp9", "h264"]);
    assert_eq!(updated.max_quality, None);
    assert_eq!(updated.default_quality, Some(720));
    assert_eq!(updated.home_sections, Some(vec!["subscriptions".into()]));
    assert_eq!(updated.audio_language.as_deref(), Some("pt-BR"));

    let updated = settings
        .update(
            user.id,
            &SettingsUpdate {
                video_codecs: Some(vec![]),
                home_sections: Some(Some(vec![])),
                audio_language: Some(None),
                ..Default::default()
            },
        )
        .await
        .unwrap();
    assert_eq!(updated.video_codecs, Vec::<String>::new());
    assert_eq!(updated.audio_language, None);
    assert_eq!(
        updated.home_sections,
        Some(vec![]),
        "empty is not the default"
    );
    assert_eq!(settings.get(user.id).await.unwrap(), updated);
}

#[tokio::test]
#[ignore = "requires Docker or OXISTREAM_TEST_DATABASE_URL"]
async fn user_repository_deletes_and_keeps_the_last_admin() {
    let db = isolated_test_pool().await;
    let users = PgUserRepository::new(db.pool.clone());
    let history = PgHistoryRepository::new(db.pool.clone());
    let admin = users
        .create(&NewUser {
            username: username(),
            password_hash: Some("hash".into()),
        })
        .await
        .unwrap();
    assert_eq!(admin.role, Role::Admin);
    assert!(admin.has_password);
    let member = users
        .provision(
            &Identity {
                issuer: "https://idp.test".into(),
                subject: unique_suffix(),
            },
            &|attempt| format!("sso_{attempt}_{}", unique_suffix()),
            false,
        )
        .await
        .unwrap();
    assert!(!member.has_password);
    history
        .record(
            member.id,
            &ProgressUpdate {
                video_id: "aaaaaaaaaaa".into(),
                position_seconds: 10.0,
                duration_seconds: 100.0,
                finished: false,
            },
        )
        .await
        .unwrap();

    assert_eq!(
        users.password_hash(admin.id).await.unwrap().unwrap(),
        "hash"
    );
    assert!(users.set_password(admin.id, "new").await.unwrap());
    assert_eq!(users.password_hash(admin.id).await.unwrap().unwrap(), "new");
    assert!(
        !users.set_password(member.id, "new").await.unwrap(),
        "single sign-on accounts never get a password"
    );
    assert_eq!(users.password_hash(member.id).await.unwrap(), None);

    assert_eq!(
        users.delete(admin.id).await.unwrap(),
        DeleteOutcome::LastAdmin
    );
    assert_eq!(
        users.delete(member.id).await.unwrap(),
        DeleteOutcome::Deleted
    );
    assert!(users.get(member.id).await.unwrap().is_none());
    assert!(
        history.list(member.id, None, 10).await.unwrap().is_empty(),
        "owned rows cascade"
    );
    assert_eq!(
        users.delete(member.id).await.unwrap(),
        DeleteOutcome::NotFound
    );
    assert_eq!(
        users.delete(admin.id).await.unwrap(),
        DeleteOutcome::Deleted,
        "the only user may leave"
    );
}

#[tokio::test]
#[ignore = "requires Docker or OXISTREAM_TEST_DATABASE_URL"]
async fn api_reads_and_patches_settings() {
    let db = isolated_test_pool().await;
    let app = app(&db.pool);
    let token = register(&app, &username()).await.token();

    let anonymous = send(&app, Method::GET, "/api/v0/settings", None, None).await;
    assert_eq!(anonymous.status, StatusCode::UNAUTHORIZED);

    let initial = send(&app, Method::GET, "/api/v0/settings", None, Some(&token)).await;
    assert_eq!(initial.status, StatusCode::OK);
    assert_eq!(
        initial.json,
        json!({
            "video_codecs": [],
            "max_quality": null,
            "default_quality": null,
            "home_sections": null,
            "audio_language": null,
        })
    );

    let patched = send(
        &app,
        Method::PATCH,
        "/api/v0/settings",
        Some(json!({
            "video_codecs": ["av1", "h264"],
            "max_quality": 1440,
            "home_sections": ["trending-news", "subscriptions"],
        })),
        Some(&token),
    )
    .await;
    assert_eq!(patched.status, StatusCode::OK, "{}", patched.json);
    assert_eq!(patched.json["video_codecs"], json!(["av1", "h264"]));
    assert_eq!(patched.json["max_quality"], 1440);

    let reset = send(
        &app,
        Method::PATCH,
        "/api/v0/settings",
        Some(json!({ "max_quality": null, "default_quality": 720, "audio_language": "interface" })),
        Some(&token),
    )
    .await;
    assert_eq!(
        reset.json,
        json!({
            "video_codecs": ["av1", "h264"],
            "max_quality": null,
            "default_quality": 720,
            "home_sections": ["trending-news", "subscriptions"],
            "audio_language": "interface",
        })
    );

    for body in [
        json!({ "max_quality": 1000 }),
        json!({ "audio_language": "German" }),
        json!({ "audio_language": "" }),
        json!({ "video_codecs": ["vp9", "vp9"] }),
        json!({ "home_sections": ["a", "a"] }),
        json!({ "home_sections": ["Not Valid"] }),
    ] {
        let reply = send(
            &app,
            Method::PATCH,
            "/api/v0/settings",
            Some(body.clone()),
            Some(&token),
        )
        .await;
        assert_eq!(reply.status, StatusCode::UNPROCESSABLE_ENTITY, "{body}");
        assert_eq!(reply.json["code"], "validation_failed");
    }
    for body in [json!({ "video_codecs": ["mpeg2"] }), json!({ "bogus": 1 })] {
        let reply = send(
            &app,
            Method::PATCH,
            "/api/v0/settings",
            Some(body.clone()),
            Some(&token),
        )
        .await;
        assert!(reply.status.is_client_error(), "{body}: {}", reply.status);
    }
}

#[tokio::test]
#[ignore = "requires Docker or OXISTREAM_TEST_DATABASE_URL"]
async fn api_changes_the_password_and_ends_other_sessions() {
    let db = isolated_test_pool().await;
    let app = app(&db.pool);
    let name = username();
    let first = register(&app, &name).await;
    assert_eq!(first.json["has_password"], true);
    let token = first.token();
    let other = login(&app, &name, PASSWORD).await.token();

    let wrong = send(
        &app,
        Method::POST,
        "/api/v0/account/password",
        Some(json!({ "current_password": "not the password", "new_password": NEW_PASSWORD })),
        Some(&token),
    )
    .await;
    assert_eq!(wrong.status, StatusCode::UNPROCESSABLE_ENTITY);
    assert_eq!(wrong.json["fields"]["current_password"], "mismatch");

    let short = send(
        &app,
        Method::POST,
        "/api/v0/account/password",
        Some(json!({ "current_password": PASSWORD, "new_password": "short" })),
        Some(&token),
    )
    .await;
    assert_eq!(short.json["fields"]["new_password"], "length");

    let changed = send(
        &app,
        Method::POST,
        "/api/v0/account/password",
        Some(json!({ "current_password": PASSWORD, "new_password": NEW_PASSWORD })),
        Some(&token),
    )
    .await;
    assert_eq!(changed.status, StatusCode::NO_CONTENT, "{}", changed.json);

    let me = send(&app, Method::GET, "/api/v0/auth/me", None, Some(&token)).await;
    assert_eq!(me.status, StatusCode::OK, "the current session stays");
    let me = send(&app, Method::GET, "/api/v0/auth/me", None, Some(&other)).await;
    assert_eq!(me.status, StatusCode::UNAUTHORIZED, "other sessions end");
    assert_eq!(
        login(&app, &name, PASSWORD).await.status,
        StatusCode::UNAUTHORIZED
    );
    assert_eq!(
        login(&app, &name, NEW_PASSWORD).await.status,
        StatusCode::OK
    );
}

#[tokio::test]
#[ignore = "requires Docker or OXISTREAM_TEST_DATABASE_URL"]
async fn api_deletes_accounts() {
    let db = isolated_test_pool().await;
    let app = app(&db.pool);
    let admin_name = username();
    let admin = register(&app, &admin_name).await.token();
    let member_name = username();
    let member = register(&app, &member_name).await.token();
    let second_session = login(&app, &member_name, PASSWORD).await.token();

    for (body, field) in [
        (
            json!({ "username": "someone", "password": PASSWORD }),
            "username",
        ),
        (
            json!({ "username": member_name, "password": "wrong password!" }),
            "password",
        ),
        (json!({ "username": member_name }), "password"),
    ] {
        let reply = send(
            &app,
            Method::DELETE,
            "/api/v0/account",
            Some(body),
            Some(&member),
        )
        .await;
        assert_eq!(reply.status, StatusCode::UNPROCESSABLE_ENTITY);
        assert_eq!(reply.json["fields"][field], "mismatch");
    }

    let last_admin = send(
        &app,
        Method::DELETE,
        "/api/v0/account",
        Some(json!({ "username": admin_name, "password": PASSWORD })),
        Some(&admin),
    )
    .await;
    assert_eq!(last_admin.status, StatusCode::CONFLICT);
    assert_eq!(last_admin.json["code"], "last_admin");

    let deleted = send(
        &app,
        Method::DELETE,
        "/api/v0/account",
        Some(json!({ "username": member_name.to_uppercase(), "password": PASSWORD })),
        Some(&member),
    )
    .await;
    assert_eq!(deleted.status, StatusCode::NO_CONTENT, "{}", deleted.json);
    assert!(deleted.set_cookie.unwrap().contains("Max-Age=0"));
    for token in [&member, &second_session] {
        let me = send(&app, Method::GET, "/api/v0/auth/me", None, Some(token)).await;
        assert_eq!(me.status, StatusCode::UNAUTHORIZED);
    }
    assert_eq!(
        login(&app, &member_name, PASSWORD).await.status,
        StatusCode::UNAUTHORIZED
    );

    let alone = send(
        &app,
        Method::DELETE,
        "/api/v0/account",
        Some(json!({ "username": admin_name, "password": PASSWORD })),
        Some(&admin),
    )
    .await;
    assert_eq!(
        alone.status,
        StatusCode::NO_CONTENT,
        "the last admin may leave once alone"
    );
}

#[tokio::test]
#[ignore = "requires Docker or OXISTREAM_TEST_DATABASE_URL"]
async fn api_handles_single_sign_on_accounts() {
    let db = isolated_test_pool().await;
    let app = app(&db.pool);
    register(&app, &username()).await;
    let user = PgUserRepository::new(db.pool.clone())
        .provision(
            &Identity {
                issuer: "https://idp.test".into(),
                subject: unique_suffix(),
            },
            &|attempt| format!("sso_{attempt}_{}", unique_suffix()),
            false,
        )
        .await
        .unwrap();
    let (token, token_hash) = cookie::new_token();
    PgSessionRepository::new(db.pool.clone())
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

    let me = send(&app, Method::GET, "/api/v0/auth/me", None, Some(&token)).await;
    assert_eq!(me.json["has_password"], false);
    let change = send(
        &app,
        Method::POST,
        "/api/v0/account/password",
        Some(json!({ "current_password": "", "new_password": NEW_PASSWORD })),
        Some(&token),
    )
    .await;
    assert_eq!(change.status, StatusCode::CONFLICT);
    assert_eq!(change.json["code"], "no_password");

    let deleted = send(
        &app,
        Method::DELETE,
        "/api/v0/account",
        Some(json!({ "username": user.username })),
        Some(&token),
    )
    .await;
    assert_eq!(deleted.status, StatusCode::NO_CONTENT, "{}", deleted.json);
}
