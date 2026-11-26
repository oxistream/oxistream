//! The database tests need `OXISTREAM_TEST_DATABASE_URL` or a Docker daemon, the live test the
//! network; run with `cargo test --test sponsorblock -- --include-ignored`.

mod common;

use std::collections::HashMap;
use std::sync::{Arc, Mutex};

use axum::Router;
use axum::body::Body;
use axum::extract::{Path, Query, State};
use axum::http::{Method, Request, StatusCode, header};
use axum::response::{IntoResponse, Response};
use axum::routing::get;
use common::{
    get as get_text, isolated_test_pool, spawn_upstream, state_without_db, unique_suffix,
};
use http_body_util::BodyExt;
use oxistream_db::{NewSession, NewUser, PgPool, PgSessionRepository, PgUserRepository};
use oxistream_db::{SessionRepository, UserRepository};
use oxistream_server::auth::cookie;
use oxistream_server::config::SponsorBlockConfig;
use oxistream_server::sponsorblock::{SponsorBlock, hash_prefix};
use oxistream_server::{AppState, app};
use serde_json::{Value, json};
use time::OffsetDateTime;
use tower::ServiceExt;

const VIDEO: &str = "aqz-KE-bpKQ";
const NEIGHBOUR: &str = "noIskSctsQ4";
const EMPTY: &str = "dQw4w9WgXcQ";
const FAILING: &str = "jNQXAC9IVRw";

/// Hash prefix, query and User-Agent of an upstream request.
type Seen = (String, HashMap<String, String>, Option<String>);

#[derive(Clone, Default)]
struct Upstream {
    requests: Arc<Mutex<Vec<Seen>>>,
}

impl Upstream {
    fn requests(&self) -> Vec<Seen> {
        self.requests.lock().unwrap().clone()
    }
}

async fn skip_segments(
    State(upstream): State<Upstream>,
    Path(prefix): Path<String>,
    Query(query): Query<HashMap<String, String>>,
    headers: axum::http::HeaderMap,
) -> Response {
    let agent = headers
        .get(header::USER_AGENT)
        .and_then(|v| v.to_str().ok())
        .map(str::to_owned);
    upstream
        .requests
        .lock()
        .unwrap()
        .push((prefix.clone(), query, agent));
    if prefix == hash_prefix(FAILING) {
        return StatusCode::INTERNAL_SERVER_ERROR.into_response();
    }
    if prefix == hash_prefix(EMPTY) {
        return (StatusCode::NOT_FOUND, "Not Found").into_response();
    }
    axum::Json(json!([
        { "videoID": NEIGHBOUR, "segments": [
            { "segment": [1.0, 2.0], "UUID": "n1", "category": "sponsor", "actionType": "skip",
              "locked": 0, "votes": 0, "videoDuration": 100.0, "description": "" }
        ]},
        { "videoID": VIDEO, "segments": [
            { "segment": [120.5, 150.0], "UUID": "s2", "category": "selfpromo", "actionType": "skip",
              "locked": 0, "votes": 3, "videoDuration": 600.0, "description": "" },
            { "segment": [10.0, 42.25], "UUID": "s1", "category": "sponsor", "actionType": "skip",
              "locked": 1, "votes": 10, "videoDuration": 600.0, "description": "" },
            { "segment": [300.0, 300.0], "UUID": "h1", "category": "poi_highlight",
              "actionType": "poi", "locked": 0, "votes": 1, "videoDuration": 600.0,
              "description": "" },
            { "segment": [0.0, 0.0], "UUID": "f1", "category": "exclusive_access",
              "actionType": "full", "locked": 0, "votes": 1, "videoDuration": 0,
              "description": "" }
        ]}
    ]))
    .into_response()
}

async fn mocked(enabled: bool) -> (Router, Upstream) {
    let upstream = Upstream::default();
    let base = spawn_upstream(
        Router::new()
            .route("/api/skipSegments/{prefix}", get(skip_segments))
            .with_state(upstream.clone()),
    )
    .await;
    let sponsorblock = SponsorBlock::new(&SponsorBlockConfig {
        enabled,
        api_url: base.as_str().trim_end_matches('/').to_owned(),
    })
    .unwrap();
    let router = app(AppState {
        sponsorblock,
        ..state_without_db()
    });
    (router, upstream)
}

async fn get_json(router: &Router, uri: &str) -> (StatusCode, Value) {
    let (status, body) = get_text(router.clone(), uri).await;
    (status, serde_json::from_str(&body).unwrap_or(Value::Null))
}

#[tokio::test]
async fn segments_are_looked_up_by_hash_prefix() {
    let (router, upstream) = mocked(true).await;
    let (status, body) = get_json(&router, &format!("/api/v0/videos/{VIDEO}/segments")).await;
    assert_eq!(status, StatusCode::OK);
    assert_eq!(
        body,
        json!({ "segments": [
            { "id": "f1", "category": "exclusive_access", "action": "full", "start": 0.0, "end": 0.0 },
            { "id": "s1", "category": "sponsor", "action": "skip", "start": 10.0, "end": 42.25 },
            { "id": "s2", "category": "selfpromo", "action": "skip", "start": 120.5, "end": 150.0 },
            { "id": "h1", "category": "poi_highlight", "action": "poi", "start": 300.0, "end": 300.0 },
        ]})
    );

    let requests = upstream.requests();
    assert_eq!(requests.len(), 1);
    let (prefix, query, agent) = &requests[0];
    assert_eq!(prefix, "671e");
    assert!(
        !query.values().any(|v| v.contains(VIDEO)),
        "the video ID never leaves the server"
    );
    let categories: Vec<String> = serde_json::from_str(&query["categories"]).unwrap();
    for category in ["sponsor", "selfpromo", "poi_highlight", "exclusive_access"] {
        assert!(categories.iter().any(|c| c == category), "{category}");
    }
    let actions: Vec<String> = serde_json::from_str(&query["actionTypes"]).unwrap();
    assert_eq!(actions, ["skip", "poi", "full"]);
    assert!(agent.as_deref().unwrap().starts_with("Oxistream/"));

    // Cached: the second lookup does not reach upstream.
    let (status, _) = get_json(&router, &format!("/api/v0/videos/{VIDEO}/segments")).await;
    assert_eq!(status, StatusCode::OK);
    assert_eq!(upstream.requests().len(), 1);
}

#[tokio::test]
async fn upstream_errors_yield_no_segments() {
    let (router, upstream) = mocked(true).await;
    for id in [EMPTY, FAILING] {
        let (status, body) = get_json(&router, &format!("/api/v0/videos/{id}/segments")).await;
        assert_eq!(status, StatusCode::OK, "{id}");
        assert_eq!(body, json!({ "segments": [] }), "{id}");
    }
    assert_eq!(upstream.requests().len(), 2);
}

#[tokio::test]
async fn disabled_sponsorblock_never_calls_upstream() {
    let (router, upstream) = mocked(false).await;
    let (status, body) = get_json(&router, &format!("/api/v0/videos/{VIDEO}/segments")).await;
    assert_eq!(status, StatusCode::OK);
    assert_eq!(body, json!({ "segments": [] }));
    assert_eq!(upstream.requests().len(), 0);

    let (_, instance) = get_json(&router, "/api/v0/instance").await;
    assert_eq!(instance["sponsorblock"], json!(false));
    let (router, _) = mocked(true).await;
    let (_, instance) = get_json(&router, "/api/v0/instance").await;
    assert_eq!(instance["sponsorblock"], json!(true));
}

#[tokio::test]
async fn invalid_video_id_is_rejected() {
    let (router, upstream) = mocked(true).await;
    let (status, _) = get_json(&router, "/api/v0/videos/bad!/segments").await;
    assert_eq!(status, StatusCode::BAD_REQUEST);
    assert_eq!(upstream.requests().len(), 0);
}

struct Api {
    app: Router,
    token: String,
}

impl Api {
    async fn new(pool: &PgPool) -> Self {
        let user = PgUserRepository::new(pool.clone())
            .create(&NewUser {
                username: format!("u_{}", unique_suffix()),
                password_hash: None,
            })
            .await
            .unwrap();
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
        Self {
            app: common::app_with_pool(pool.clone()),
            token,
        }
    }

    async fn request(
        &self,
        method: Method,
        body: Option<&str>,
        signed_in: bool,
    ) -> (StatusCode, Value) {
        let mut req = Request::builder()
            .method(method)
            .uri("/api/v0/settings/sponsorblock")
            .header(header::HOST, "tube.test")
            .header(header::ORIGIN, "http://tube.test")
            .header(header::CONTENT_TYPE, "application/json");
        if signed_in {
            req = req.header(header::COOKIE, format!("oxistream_session={}", self.token));
        }
        let body = body.map_or_else(Body::empty, |b| Body::from(b.to_owned()));
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

fn defaults() -> Value {
    json!({
        "categories": {
            "sponsor": "auto",
            "selfpromo": "button",
            "interaction": "button",
            "intro": "button",
            "outro": "button",
            "preview": "button",
            "hook": "button",
            "music_offtopic": "button",
            "filler": "button",
        },
        "highlight": true,
        "full_video_label": true,
    })
}

#[tokio::test]
#[ignore = "requires Docker or OXISTREAM_TEST_DATABASE_URL"]
async fn settings_round_trip() {
    let db = isolated_test_pool().await;
    let api = Api::new(&db.pool).await;

    let (status, body) = api.request(Method::GET, None, true).await;
    assert_eq!(status, StatusCode::OK);
    assert_eq!(body, defaults());

    let mut changed = defaults();
    changed["categories"]["sponsor"] = json!("button");
    changed["categories"]["filler"] = json!("off");
    changed["highlight"] = json!(false);
    let (status, body) = api
        .request(Method::PUT, Some(&changed.to_string()), true)
        .await;
    assert_eq!(status, StatusCode::OK);
    assert_eq!(body, changed);
    let (_, body) = api.request(Method::GET, None, true).await;
    assert_eq!(body, changed);

    // Missing fields take their defaults.
    let (status, body) = api
        .request(
            Method::PUT,
            Some(r#"{"categories":{"intro":"auto"}}"#),
            true,
        )
        .await;
    assert_eq!(status, StatusCode::OK);
    let mut expected = defaults();
    expected["categories"]["intro"] = json!("auto");
    assert_eq!(body, expected);

    let (status, _) = api
        .request(
            Method::PUT,
            Some(r#"{"categories":{"sponsor":"sometimes"}}"#),
            true,
        )
        .await;
    assert!(status.is_client_error(), "{status}");
    let (_, body) = api.request(Method::GET, None, true).await;
    assert_eq!(body, expected, "a rejected update changes nothing");
}

#[tokio::test]
#[ignore = "requires Docker or OXISTREAM_TEST_DATABASE_URL"]
async fn settings_need_an_account() {
    let db = isolated_test_pool().await;
    let api = Api::new(&db.pool).await;
    let (status, _) = api.request(Method::GET, None, false).await;
    assert_eq!(status, StatusCode::UNAUTHORIZED);
    let (status, _) = api
        .request(Method::PUT, Some(&defaults().to_string()), false)
        .await;
    assert_eq!(status, StatusCode::UNAUTHORIZED);
}

#[tokio::test]
#[ignore = "network"]
async fn live_segments() {
    let sponsorblock = SponsorBlock::new(&SponsorBlockConfig::default()).unwrap();
    // A video with long-standing sponsor and intro segments.
    let segments = sponsorblock.segments("h7Qq2BXUM2o").await;
    assert!(
        segments
            .iter()
            .any(|s| s.category == oxistream_server::sponsorblock::Category::Sponsor),
        "{segments:?}"
    );
}
