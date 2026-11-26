mod common;

use axum::http::StatusCode;
use common::{app_without_db, get};

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
