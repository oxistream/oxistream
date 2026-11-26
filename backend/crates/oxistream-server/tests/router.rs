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
