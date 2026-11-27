//! Talks to YouTube; run with `cargo test --test youtube_live -- --ignored`.

mod common;

use std::sync::Arc;

use axum::body::Body;
use axum::http::{Request, StatusCode, header};
use common::{app_without_db, get, state_without_db};
use http_body_util::BodyExt;
use oxistream_server::config::YouTubeConfig;
use oxistream_server::youtube::YouTube;
use rustypipe::param::{Country, Language, TrendingCategory};
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
    let items = page["items"].as_array().unwrap();
    assert_ne!(items.len(), 0);
    let videos: Vec<Value> = items
        .iter()
        .filter(|item| item["type"] == "video")
        .cloned()
        .collect();
    assert_video_metadata(&Value::Array(videos));
}

#[tokio::test]
#[ignore = "network"]
async fn search_types_paginate() {
    for (kind, item_type) in [
        ("videos", "video"),
        ("shorts", "video"),
        ("channels", "channel"),
        ("playlists", "playlist"),
    ] {
        let (status, json) = get_json(&format!("/api/v0/search?q=cats&type={kind}")).await;
        assert_eq!(status, StatusCode::OK, "{kind}: {json}");
        let items = json["items"].as_array().unwrap();
        assert_ne!(items.len(), 0, "{kind}");
        assert!(items.iter().all(|item| item["type"] == item_type), "{kind}");
        if kind == "shorts" {
            assert!(items.iter().all(|item| item["is_short"] == true), "{json}");
        }

        let token = json["continuation"].as_str().expect("continuation token");
        let (status, page) =
            get_json(&format!("/api/v0/search?continuation={token}&type={kind}")).await;
        assert_eq!(status, StatusCode::OK, "{kind}: {page}");
        let items = page["items"].as_array().unwrap();
        assert_ne!(items.len(), 0, "{kind}");
        assert!(items.iter().all(|item| item["type"] == item_type), "{kind}");
    }
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

const LONG_PLAYLIST_ID: &str = "PL4lEESSgxM_5O81EvKCmBIm_JT5Q7JeaI";

#[tokio::test]
#[ignore = "network"]
async fn playlist_paginates() {
    let (status, json) = get_json(&format!("/api/v0/youtube/playlists/{LONG_PLAYLIST_ID}")).await;
    assert_eq!(status, StatusCode::OK, "{json}");
    assert_eq!(json["id"], LONG_PLAYLIST_ID);
    assert!(json["title"].as_str().is_some_and(|t| !t.is_empty()));
    assert!(json["video_count"].as_u64().is_some_and(|n| n > 100));
    assert!(
        json["channel"]["id"]
            .as_str()
            .is_some_and(|id| id.starts_with("UC"))
    );
    assert!(json["last_update"].as_str().is_some());
    let thumbnail = json["thumbnails"][0]["url"].as_str().unwrap();
    assert!(thumbnail.starts_with("/proxy/image/"), "{thumbnail}");
    let first = json["videos"]["items"].as_array().unwrap();
    assert_eq!(first.len(), 100);
    let token = json["videos"]["continuation"]
        .as_str()
        .expect("continuation");

    let (status, page) = get_json(&format!(
        "/api/v0/youtube/playlists/{LONG_PLAYLIST_ID}/videos?continuation={token}"
    ))
    .await;
    assert_eq!(status, StatusCode::OK, "{page}");
    let next = page["items"].as_array().unwrap();
    assert_ne!(next.len(), 0);
    assert_ne!(next[0]["id"], first[0]["id"]);
    assert_video_metadata(&page["items"]);
}

#[tokio::test]
#[ignore = "network"]
async fn followed_playlist_fetch_follows_continuations() {
    use oxistream_server::feed::{MAX_STORED_PLAYLIST_ITEMS, PlaylistSource};

    let youtube = YouTube::new(&YouTubeConfig::default()).unwrap();
    let fetch = youtube.fetch_playlist(LONG_PLAYLIST_ID).await.unwrap();
    assert_eq!(fetch.playlist.id, LONG_PLAYLIST_ID);
    assert!(fetch.items.len() > 100, "{}", fetch.items.len());
    assert!(fetch.items.len() <= MAX_STORED_PLAYLIST_ITEMS);
    assert!(fetch.items.iter().all(|i| i.video_id.len() == 11));
}

async fn assert_paginates(path: &str) -> Value {
    let (status, json) = get_json(path).await;
    assert_eq!(status, StatusCode::OK, "{path}: {json}");
    assert_ne!(json["items"].as_array().unwrap().len(), 0, "{path}");
    let token = json["continuation"].as_str().expect("continuation");
    let sep = if path.contains('?') { '&' } else { '?' };
    let (status, page) = get_json(&format!("{path}{sep}continuation={token}")).await;
    assert_eq!(status, StatusCode::OK, "{path}: {page}");
    assert_ne!(page["items"].as_array().unwrap().len(), 0, "{path}");
    json
}

#[tokio::test]
#[ignore = "network"]
async fn channel_tabs_paginate() {
    let (_, channel) = get_json(&format!("/api/v0/channels/{CHANNEL_ID}")).await;
    assert_eq!(channel["has_shorts"], true, "{channel}");
    assert_eq!(channel["has_live"], true, "{channel}");

    let shorts = assert_paginates(&format!("/api/v0/channels/{CHANNEL_ID}/shorts")).await;
    assert!(
        shorts["items"]
            .as_array()
            .unwrap()
            .iter()
            .all(|v| v["is_short"] == true),
        "{shorts:#}"
    );
    assert_paginates(&format!("/api/v0/channels/{CHANNEL_ID}/live")).await;
    assert_paginates(&format!(
        "/api/v0/channels/{CHANNEL_ID}/shorts?sort=popular"
    ))
    .await;
}

#[tokio::test]
#[ignore = "network"]
async fn channel_videos_follow_sort() {
    let first = |json: &Value| json["items"][0]["id"].as_str().unwrap().to_owned();
    let latest = assert_paginates(&format!("/api/v0/channels/{CHANNEL_ID}/videos")).await;
    let popular = assert_paginates(&format!(
        "/api/v0/channels/{CHANNEL_ID}/videos?sort=popular"
    ))
    .await;
    let oldest =
        assert_paginates(&format!("/api/v0/channels/{CHANNEL_ID}/videos?sort=oldest")).await;
    assert_ne!(first(&latest), first(&oldest));
    assert_ne!(first(&latest), first(&popular));
}

#[tokio::test]
#[ignore = "network"]
async fn channel_playlists_paginate() {
    let json = assert_paginates(&format!("/api/v0/channels/{CHANNEL_ID}/playlists")).await;
    let playlist = &json["items"][0];
    assert!(playlist["id"].as_str().is_some_and(|id| !id.is_empty()));
    assert!(playlist["title"].as_str().is_some_and(|t| !t.is_empty()));
    let url = playlist["thumbnails"][0]["url"].as_str().unwrap();
    assert!(url.starts_with("/proxy/image/"), "{url}");
}

#[tokio::test]
#[ignore = "network"]
async fn channel_search_paginates() {
    assert_paginates(&format!("/api/v0/channels/{CHANNEL_ID}/search?q=gpu")).await;
}

#[tokio::test]
#[ignore = "network"]
async fn unknown_playlist_is_not_found() {
    let (status, json) =
        get_json("/api/v0/youtube/playlists/PLzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzz").await;
    assert_eq!(status, StatusCode::NOT_FOUND, "{json}");
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

#[tokio::test]
#[ignore = "network"]
async fn trending_lists_proxied_videos_per_category() {
    for category in ["podcasts", "gaming", "news"] {
        let (status, json) = get_json(&format!("/api/v0/trending?category={category}")).await;
        assert_eq!(status, StatusCode::OK, "{json}");
        assert_eq!(json["region"], "US");
        assert_eq!(json["category"], category);
        let items = json["items"].as_array().unwrap();
        assert!(items.len() >= 20, "{category}: {} items", items.len());
        let thumb = items[0]["thumbnails"][0]["url"].as_str().unwrap();
        assert!(thumb.starts_with("/proxy/image/"), "{thumb}");
    }
    let (status, json) = get_json("/api/v0/trending").await;
    assert_eq!(status, StatusCode::OK, "{json}");
    assert_eq!(json["category"], "podcasts");
}

#[tokio::test]
#[ignore = "network"]
async fn trending_follows_configured_region() {
    let config = YouTubeConfig {
        trending_region: Country::De,
        ..YouTubeConfig::default()
    };
    let youtube = YouTube::new(&config).unwrap();
    let first = youtube
        .trending(TrendingCategory::Gaming, Language::De)
        .await
        .unwrap();
    assert!(first.len() >= 20, "{} items", first.len());
    let cached = youtube
        .trending(TrendingCategory::Gaming, Language::De)
        .await
        .unwrap();
    assert!(Arc::ptr_eq(&first, &cached));
}

#[tokio::test]
#[ignore = "network"]
async fn shorts_discovery_paginates() {
    let (status, json) = get_json("/api/v0/shorts/discover").await;
    assert_eq!(status, StatusCode::OK, "{json}");
    let items = json["items"].as_array().unwrap();
    assert!(items.len() >= 3, "{json}");
    for item in items {
        assert!(
            item["id"].as_str().is_some_and(|id| id.len() == 11),
            "{item}"
        );
        assert!(
            item["channel_id"]
                .as_str()
                .is_some_and(|id| id.starts_with("UC"))
        );
        let thumb = item["thumbnails"][0]["url"].as_str().unwrap();
        assert!(thumb.starts_with("/proxy/image/"), "{thumb}");
    }
    let token = json["continuation"].as_str().expect("continuation token");

    let (status, page) = get_json(&format!("/api/v0/shorts/discover?continuation={token}")).await;
    assert_eq!(status, StatusCode::OK, "{page}");
    assert_ne!(page["items"].as_array().unwrap().len(), 0, "{page}");
    assert!(page["continuation"].is_string());
}

#[tokio::test]
#[ignore = "network"]
async fn short_details_link_the_full_video() {
    let (status, json) = get_json("/api/v0/shorts/sHpEqeSByPI").await;
    assert_eq!(status, StatusCode::OK, "{json}");
    assert_eq!(json["linked_video"]["id"], "q6fgVVmdpgY");
    assert!(
        json["linked_video"]["title"]
            .as_str()
            .is_some_and(|t| !t.is_empty())
    );

    let (status, json) = get_json("/api/v0/shorts/kQO5P-qsyiA").await;
    assert_eq!(status, StatusCode::OK, "{json}");
    assert!(json["linked_video"].is_null(), "{json}");
}

fn has_disclosure(json: &Value, kind: &str) -> bool {
    json["content_disclosures"]
        .as_array()
        .unwrap()
        .iter()
        .any(|d| d["kind"] == kind && d["text"].as_str().is_some_and(|t| !t.is_empty()))
}

#[tokio::test]
#[ignore = "network"]
async fn shows_content_disclosures() {
    let (status, json) = get_json("/api/v0/videos/-YrdcUHpNV4").await;
    assert_eq!(status, StatusCode::OK, "{json}");
    assert!(has_disclosure(&json, "altered_or_synthetic"), "{json}");

    let (status, json) = get_json("/api/v0/shorts/tTjXWrpXdW4").await;
    assert_eq!(status, StatusCode::OK, "{json}");
    assert!(has_disclosure(&json, "altered_or_synthetic"), "{json}");
}

#[tokio::test]
#[ignore = "network"]
async fn marks_auto_dubbed_audio_tracks() {
    let id = "i8_Pz7sBp_M";
    let (status, json) = get_json(&format!("/api/v0/videos/{id}")).await;
    assert_eq!(status, StatusCode::OK, "{json}");
    let tracks = json["audio_tracks"].as_array().unwrap();
    assert!(
        tracks
            .iter()
            .any(|t| t["kind"] == "original" && t["is_default"] == true),
        "{json}"
    );
    assert!(tracks.iter().any(|t| t["kind"] == "auto_dubbed"), "{json}");

    let (status, mpd) = get(
        app_without_db(),
        &format!("/api/v0/videos/{id}/manifest.mpd"),
    )
    .await;
    assert_eq!(status, StatusCode::OK);
    assert!(mpd.contains(r#"value="main""#), "{mpd}");
    assert!(mpd.contains(r#"value="auto-dub""#), "{mpd}");
}
