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
    assert_video_metadata(&page["items"]);
}

/// Most items carry duration, views and a date; single ones may legitimately lack them.
fn assert_video_metadata(items: &Value) {
    let items = items.as_array().unwrap();
    for field in ["duration_seconds", "view_count", "published_text"] {
        let present = items.iter().filter(|v| !v[field].is_null()).count();
        assert!(
            present * 5 >= items.len() * 4,
            "{field} missing in {items:#?}"
        );
    }
}

const MKBHD_ID: &str = "UCBJycsmduvYEL83R_U4JriQ";

#[tokio::test]
#[ignore = "network"]
async fn channel_reports_subscribers_and_video_metadata() {
    for lang in ["en", "de"] {
        let request = Request::get(format!("/api/v0/channels/{MKBHD_ID}"))
            .header(header::ACCEPT_LANGUAGE, lang)
            .body(Body::empty())
            .unwrap();
        let response = app_without_db().oneshot(request).await.unwrap();
        assert_eq!(response.status(), StatusCode::OK);
        let body = response.into_body().collect().await.unwrap().to_bytes();
        let json: Value = serde_json::from_slice(&body).unwrap();

        let subscribers = json["subscriber_count"].as_u64().unwrap_or_default();
        assert!(subscribers > 1_000_000, "{lang}: {subscribers} subscribers");
        let videos = json["video_count"].as_u64().unwrap_or_default();
        assert!(
            (100..subscribers / 100).contains(&videos),
            "{lang}: {videos} videos"
        );
        assert_video_metadata(&json["videos"]["items"]);
    }
}

#[tokio::test]
#[ignore = "network"]
async fn related_videos_carry_metadata() {
    let (status, json) = get_json(&format!("/api/v0/videos/{VIDEO_ID}")).await;
    assert_eq!(status, StatusCode::OK, "{json}");
    let related = &json["related"];
    assert_video_metadata(related);
    let with_channel = related
        .as_array()
        .unwrap()
        .iter()
        .filter(|v| {
            v["channel"]["id"]
                .as_str()
                .is_some_and(|id| id.starts_with("UC"))
        })
        .count();
    assert!(with_channel > 0, "no channels in {related:#?}");
}

#[tokio::test]
#[ignore = "network"]
async fn live_items_report_viewers_not_views() {
    let (status, json) = get_json("/api/v0/search?q=lofi+hip+hop+radio+live").await;
    assert_eq!(status, StatusCode::OK, "{json}");
    let live: Vec<&Value> = json["items"]
        .as_array()
        .unwrap()
        .iter()
        .filter(|item| item["is_live"] == true)
        .collect();
    assert_ne!(live.len(), 0, "no live items in {json:#}");
    for item in live {
        assert!(item["view_count"].is_null(), "{item:#}");
        assert!(item["viewers"].as_u64().is_some(), "{item:#}");
    }
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

#[tokio::test]
#[ignore = "network"]
async fn manifest_points_to_proxy() {
    let router = oxistream_server::app(state_without_db());
    let response = fetch(
        router.clone(),
        &format!("/api/v0/videos/{VIDEO_ID}/manifest.mpd"),
        None,
    )
    .await;
    assert_eq!(response.status(), StatusCode::OK);
    assert_eq!(
        response.headers()[header::CONTENT_TYPE],
        "application/dash+xml"
    );
    let body = response.into_body().collect().await.unwrap().to_bytes();
    let mpd = String::from_utf8(body.to_vec()).unwrap();

    let urls: Vec<String> = mpd
        .split("<BaseURL>")
        .skip(1)
        .map(|rest| {
            rest.split("</BaseURL>")
                .next()
                .unwrap()
                .replace("&amp;", "&")
        })
        .collect();
    assert!(urls.len() > 2, "{mpd}");
    for url in &urls {
        assert!(
            url.starts_with(&format!("/proxy/stream/{VIDEO_ID}/")),
            "{url}"
        );
        assert!(url.contains("exp=") && url.contains("&sig="), "{url}");
    }

    let response = fetch(router, &urls[0], Some("bytes=0-499")).await;
    assert_eq!(response.status(), StatusCode::PARTIAL_CONTENT);
    let body = response.into_body().collect().await.unwrap().to_bytes();
    assert_eq!(body.len(), 500);
}

#[tokio::test]
#[ignore = "network"]
async fn video_details_list_proxied_subtitles() {
    let (status, json) = get_json(&format!("/api/v0/videos/{VIDEO_ID}")).await;
    assert_eq!(status, StatusCode::OK, "{json}");
    let subtitles = json["subtitles"].as_array().unwrap();
    assert_ne!(subtitles.len(), 0);
    let url = subtitles[0]["url"].as_str().unwrap();
    assert!(
        url.starts_with(&format!("/proxy/subtitle/{VIDEO_ID}/")),
        "{url}"
    );

    let response = fetch(app_without_db(), url, None).await;
    assert_eq!(response.status(), StatusCode::OK);
    let body = response.into_body().collect().await.unwrap().to_bytes();
    assert!(
        body.starts_with(b"WEBVTT"),
        "{}",
        String::from_utf8_lossy(&body)
    );
}
