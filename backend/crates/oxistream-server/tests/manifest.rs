//! DASH manifest endpoint with fixture player data.

mod common;

use std::sync::Arc;

use async_trait::async_trait;
use axum::Router;
use axum::body::Body;
use axum::http::{Request, StatusCode, header};
use common::{fake_player, get, state_without_db};
use oxistream_server::config::YouTubeConfig;
use oxistream_server::youtube::{self, PlayerSource, YouTube};
use oxistream_server::{AppState, app};
use rustypipe::model::VideoPlayer;
use tower::ServiceExt;

const VIDEO_ID: &str = "dQw4w9WgXcQ";
const LIVE_ID: &str = "jfKfPfyJRdk";

struct Fixture;

#[async_trait]
impl PlayerSource for Fixture {
    async fn player(&self, video_id: &str) -> youtube::Result<VideoPlayer> {
        let mut player = fake_player("https://rr1---sn-test.googlevideo.com/", video_id);
        player.details.is_live = video_id == LIVE_ID;
        Ok(player)
    }
}

fn router() -> Router {
    app(AppState {
        youtube: YouTube::with_player_source(&YouTubeConfig::default(), Arc::new(Fixture)).unwrap(),
        ..state_without_db()
    })
}

fn base_urls(mpd: &str) -> Vec<String> {
    mpd.split("<BaseURL>")
        .skip(1)
        .map(|rest| {
            rest.split("</BaseURL>")
                .next()
                .unwrap()
                .replace("&amp;", "&")
        })
        .collect()
}

#[tokio::test]
async fn serves_manifest_with_proxied_urls() {
    let request = Request::get(format!("/api/v0/videos/{VIDEO_ID}/manifest.mpd"))
        .body(Body::empty())
        .unwrap();
    let response = router().oneshot(request).await.unwrap();
    assert_eq!(response.status(), StatusCode::OK);
    assert_eq!(
        response.headers()[header::CONTENT_TYPE],
        "application/dash+xml"
    );
    assert!(response.headers().contains_key(header::CACHE_CONTROL));

    let (_, mpd) = get(router(), &format!("/api/v0/videos/{VIDEO_ID}/manifest.mpd")).await;
    assert!(
        mpd.contains(r#"mediaPresentationDuration="PT213S""#),
        "{mpd}"
    );
    assert!(
        mpd.contains(r#"lang="en""#) && mpd.contains(r#"lang="de""#),
        "{mpd}"
    );
    assert!(mpd.contains("TransferCharacteristics"), "{mpd}");

    let urls = base_urls(&mpd);
    assert_eq!(urls.len(), 4, "{mpd}");
    for url in &urls {
        assert!(
            url.starts_with(&format!("/proxy/stream/{VIDEO_ID}/")),
            "{url}"
        );
        assert!(url.contains("exp=") && url.contains("&sig="), "{url}");
    }
    assert!(urls.iter().any(|u| u.contains("/251?track=de.3&")));
}

#[tokio::test]
async fn rejects_livestreams_and_invalid_ids() {
    let (status, body) = get(router(), &format!("/api/v0/videos/{LIVE_ID}/manifest.mpd")).await;
    assert_eq!(status, StatusCode::NOT_FOUND);
    let json: serde_json::Value = serde_json::from_str(&body).unwrap();
    assert_eq!(json["code"], "unavailable");
    assert_eq!(json["reason"], "livestream");

    let (status, _) = get(router(), "/api/v0/videos/nope/manifest.mpd").await;
    assert_eq!(status, StatusCode::BAD_REQUEST);
}
