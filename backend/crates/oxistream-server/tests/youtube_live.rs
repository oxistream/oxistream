//! Talks to YouTube; run with `cargo test --test youtube_live -- --ignored`.

mod common;

use std::sync::Arc;

use axum::body::Body;
use axum::http::{Request, StatusCode, header};
use common::{app_without_db, get, state_without_db};
use http_body_util::BodyExt;
use oxistream_server::config::YouTubeConfig;
use oxistream_server::youtube::YouTube;
use serde_json::Value;
use tower::ServiceExt;

const VIDEO_ID: &str = "dQw4w9WgXcQ";
const CHANNEL_ID: &str = "UCXuqSBlHAE6Xw-yeJA0Tunw";

async fn get_json(uri: &str) -> (StatusCode, Value) {
    let (status, body) = get(app_without_db(), uri).await;
    let json = serde_json::from_str(&body).unwrap_or_else(|_| panic!("{status}: {body}"));
    (status, json)
}

#[tokio::test]
#[ignore = "network"]
async fn video_details() {
    let (status, json) = get_json(&format!("/api/v0/videos/{VIDEO_ID}")).await;
    assert_eq!(status, StatusCode::OK, "{json}");
    assert_eq!(json["id"], VIDEO_ID);
    assert!(json["title"].as_str().is_some_and(|t| !t.is_empty()));
    assert_eq!(json["channel"]["id"], "UCuAXFkgsw1L7xaCfnd5JJOw");
    assert!(json["duration_seconds"].as_u64().is_some_and(|d| d > 0));
    assert!(json["view_count"].as_u64().is_some_and(|v| v > 0));
    assert_ne!(json["thumbnails"].as_array().unwrap().len(), 0);
    assert_ne!(json["related"].as_array().unwrap().len(), 0);
}

#[tokio::test]
#[ignore = "network"]
async fn video_details_follow_accept_language() {
    let request = Request::get(format!("/api/v0/videos/{VIDEO_ID}"))
        .header(header::ACCEPT_LANGUAGE, "de-DE,de;q=0.9,en;q=0.8")
        .body(Body::empty())
        .unwrap();
    let response = app_without_db().oneshot(request).await.unwrap();
    assert_eq!(response.status(), StatusCode::OK);
    let body = response.into_body().collect().await.unwrap().to_bytes();
    let json: Value = serde_json::from_slice(&body).unwrap();
    let published = json["published_text"].as_str().unwrap();
    assert!(published.contains("2009"), "{published}");
    assert!(
        !published.contains("Oct"),
        "expected German date, got {published}"
    );
}

#[tokio::test]
#[ignore = "network"]
async fn unknown_video_is_not_found() {
    let (status, json) = get_json("/api/v0/videos/aaaaaaaaaaa").await;
    assert_eq!(status, StatusCode::NOT_FOUND, "{json}");
}

#[tokio::test]
#[ignore = "network"]
async fn search_paginates() {
    let (status, json) = get_json("/api/v0/search?q=rust+programming").await;
    assert_eq!(status, StatusCode::OK, "{json}");
    let items = json["items"].as_array().unwrap();
    assert!(items.iter().any(|item| item["type"] == "video"));
    let token = json["continuation"].as_str().expect("continuation token");

    let (status, page) = get_json(&format!("/api/v0/search?continuation={token}")).await;
    assert_eq!(status, StatusCode::OK, "{page}");
    assert_ne!(page["items"].as_array().unwrap().len(), 0);
}

#[tokio::test]
#[ignore = "network"]
async fn search_suggestions() {
    let (status, json) = get_json("/api/v0/search/suggestions?q=rust").await;
    assert_eq!(status, StatusCode::OK, "{json}");
    assert_ne!(json.as_array().unwrap().len(), 0);
}

#[tokio::test]
#[ignore = "network"]
async fn channel_paginates() {
    let (status, json) = get_json(&format!("/api/v0/channels/{CHANNEL_ID}")).await;
    assert_eq!(status, StatusCode::OK, "{json}");
    assert_eq!(json["id"], CHANNEL_ID);
    assert_ne!(json["videos"]["items"].as_array().unwrap().len(), 0);
    let token = json["videos"]["continuation"]
        .as_str()
        .expect("continuation");

    let (status, page) = get_json(&format!(
        "/api/v0/channels/{CHANNEL_ID}/videos?continuation={token}"
    ))
    .await;
    assert_eq!(status, StatusCode::OK, "{page}");
    assert_ne!(page["items"].as_array().unwrap().len(), 0);
}

#[tokio::test]
#[ignore = "network"]
async fn resolves_urls() {
    let cases = [
        ("%2F%40LinusTechTips", "channel", CHANNEL_ID),
        ("https%3A%2F%2Fyoutu.be%2FdQw4w9WgXcQ", "video", VIDEO_ID),
        ("%2Fwatch%3Fv%3DdQw4w9WgXcQ", "video", VIDEO_ID),
        (
            "%2Fchannel%2FUCXuqSBlHAE6Xw-yeJA0Tunw",
            "channel",
            CHANNEL_ID,
        ),
        (
            "%2Fplaylist%3Flist%3DPL4lEESSgxM_5O81EvKCmBIm_JT5Q7JeaI",
            "playlist",
            "PL4lEESSgxM_5O81EvKCmBIm_JT5Q7JeaI",
        ),
    ];
    for (url, kind, id) in cases {
        let (status, json) = get_json(&format!("/api/v0/resolve?url={url}")).await;
        assert_eq!(status, StatusCode::OK, "{url}: {json}");
        assert_eq!(
            (json["kind"].as_str(), json["id"].as_str()),
            (Some(kind), Some(id))
        );
    }

    let (status, _) = get_json("/api/v0/resolve?url=https%3A%2F%2Fexample.com%2Fx").await;
    assert_eq!(status, StatusCode::BAD_REQUEST);
}

#[tokio::test]
#[ignore = "network"]
async fn player_is_cached() {
    let youtube = YouTube::new(&YouTubeConfig::default()).unwrap();
    let (a, b) = tokio::join!(youtube.player(VIDEO_ID), youtube.player(VIDEO_ID));
    let (a, b) = (a.unwrap(), b.unwrap());
    assert!(Arc::ptr_eq(&a, &b));
    assert_ne!(a.audio_streams.len(), 0);

    youtube.invalidate(VIDEO_ID).await;
    let c = youtube.player(VIDEO_ID).await.unwrap();
    assert!(!Arc::ptr_eq(&a, &c));
}

async fn fetch(router: axum::Router, uri: &str, range: Option<&str>) -> axum::response::Response {
    let mut request = Request::get(uri);
    if let Some(range) = range {
        request = request.header(header::RANGE, range);
    }
    router
        .oneshot(request.body(Body::empty()).unwrap())
        .await
        .unwrap()
}

#[tokio::test]
#[ignore = "network"]
async fn proxies_thumbnail() {
    let (status, json) = get_json(&format!("/api/v0/videos/{VIDEO_ID}")).await;
    assert_eq!(status, StatusCode::OK, "{json}");
    let url = json["thumbnails"][0]["url"].as_str().unwrap();
    assert!(url.starts_with("/proxy/image/"), "{url}");

    let response = fetch(app_without_db(), url, None).await;
    assert_eq!(response.status(), StatusCode::OK);
    let content_type = response.headers()[header::CONTENT_TYPE].to_str().unwrap();
    assert!(content_type.starts_with("image/"), "{content_type}");
    let body = response.into_body().collect().await.unwrap().to_bytes();
    assert!(body.len() > 1000);
}

#[tokio::test]
#[ignore = "network"]
async fn proxies_stream_range() {
    let state = state_without_db();
    let player = state.youtube.player(VIDEO_ID).await.unwrap();
    let audio = &player.audio_streams[0];
    let url = state.stream_url(
        VIDEO_ID,
        audio.itag,
        audio.track.as_ref().map(|t| t.id.as_str()),
    );

    let response = fetch(oxistream_server::app(state), &url, Some("bytes=0-1023")).await;
    assert_eq!(response.status(), StatusCode::PARTIAL_CONTENT);
    assert_eq!(response.headers()[header::CONTENT_LENGTH], "1024");
    let content_range = response.headers()[header::CONTENT_RANGE].to_str().unwrap();
    assert!(
        content_range.starts_with("bytes 0-1023/"),
        "{content_range}"
    );
    let body = response.into_body().collect().await.unwrap().to_bytes();
    assert_eq!(body.len(), 1024);
}
