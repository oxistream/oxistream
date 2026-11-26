mod common;

use axum::body::Body;
use axum::http::{Request, StatusCode, header};
use common::{app_without_db, get};
use tower::ServiceExt;

#[tokio::test]
async fn healthz_is_ok_without_database() {
    let (status, body) = get(app_without_db(), "/healthz").await;
    assert_eq!(status, StatusCode::OK);
    assert_eq!(body, "ok");
}

#[tokio::test]
async fn readyz_is_unavailable_without_database() {
    let (status, _) = get(app_without_db(), "/readyz").await;
    assert_eq!(status, StatusCode::SERVICE_UNAVAILABLE);
}

#[tokio::test]
async fn instance_reports_name_and_version() {
    let (status, body) = get(app_without_db(), "/api/v0/instance").await;
    assert_eq!(status, StatusCode::OK);
    let json: serde_json::Value = serde_json::from_str(&body).unwrap();
    assert_eq!(json["name"], "Oxistream");
    assert_eq!(json["version"], env!("CARGO_PKG_VERSION"));
}

#[tokio::test]
async fn serves_openapi_spec_and_docs() {
    let (status, body) = get(app_without_db(), "/api/openapi.json").await;
    assert_eq!(status, StatusCode::OK);
    let json: serde_json::Value = serde_json::from_str(&body).unwrap();
    assert!(json["paths"]["/api/v0/instance"].is_object());

    let (status, body) = get(app_without_db(), "/api/docs").await;
    assert_eq!(status, StatusCode::OK);
    assert!(body.contains("<html"));
}

#[tokio::test]
async fn unknown_api_route_is_not_found() {
    let (status, _) = get(app_without_db(), "/api/v0/does-not-exist").await;
    assert_eq!(status, StatusCode::NOT_FOUND);
}

#[tokio::test]
async fn metrics_endpoint_renders() {
    let (status, _) = get(app_without_db(), "/metrics").await;
    assert_eq!(status, StatusCode::OK);
}

async fn error_code(uri: &str) -> (StatusCode, String) {
    let (status, body) = get(app_without_db(), uri).await;
    let json: serde_json::Value = serde_json::from_str(&body).unwrap();
    (status, json["code"].as_str().unwrap().to_owned())
}

#[tokio::test]
async fn youtube_endpoints_validate_input_without_upstream() {
    for uri in [
        "/api/v0/videos/not-an-id",
        "/api/v0/channels/not-a-channel",
        "/api/v0/channels/not-a-channel/videos",
        "/api/v0/search",
        "/api/v0/search?q=%20",
        "/api/v0/resolve",
        "/api/v0/resolve?url=https%3A%2F%2Fexample.com%2Fwatch%3Fv%3DdQw4w9WgXcQ",
    ] {
        assert_eq!(
            error_code(uri).await,
            (StatusCode::BAD_REQUEST, "bad_request".to_owned()),
            "{uri}"
        );
    }
}

#[tokio::test]
async fn empty_search_suggestions() {
    let (status, body) = get(app_without_db(), "/api/v0/search/suggestions?q=").await;
    assert_eq!(status, StatusCode::OK);
    assert_eq!(body, "[]");
}

#[tokio::test]
async fn instance_reports_auth_capabilities() {
    let (_, body) = get(app_without_db(), "/api/v0/instance").await;
    let json: serde_json::Value = serde_json::from_str(&body).unwrap();
    assert_eq!(
        json["auth"],
        serde_json::json!({
            "local_enabled": true,
            "registration_enabled": true,
            "require_login": false,
            "oidc_enabled": false,
        })
    );
}

#[tokio::test]
async fn state_changing_requests_need_a_matching_origin() {
    for origin in [None, Some("https://evil.example"), Some("null")] {
        let mut req = Request::post("/api/v0/auth/logout").header(header::HOST, "tube.test");
        if let Some(origin) = origin {
            req = req.header(header::ORIGIN, origin);
        }
        let response = app_without_db()
            .oneshot(req.body(Body::empty()).unwrap())
            .await
            .unwrap();
        assert_eq!(response.status(), StatusCode::FORBIDDEN, "{origin:?}");
    }

    let response = app_without_db()
        .oneshot(
            Request::post("/api/v0/auth/logout")
                .header(header::HOST, "tube.test")
                .header(header::ORIGIN, "http://tube.test")
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(response.status(), StatusCode::NO_CONTENT);
}

#[tokio::test]
async fn me_without_session_is_unauthorized() {
    let (status, body) = get(app_without_db(), "/api/v0/auth/me").await;
    assert_eq!(status, StatusCode::UNAUTHORIZED);
    assert!(body.contains("\"unauthorized\""));
}
