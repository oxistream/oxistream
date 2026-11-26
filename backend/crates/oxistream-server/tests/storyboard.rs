//! Storyboard track endpoint with fixture player data.

mod common;

use std::sync::Arc;

use async_trait::async_trait;
use axum::Router;
use axum::http::StatusCode;
use base64::Engine;
use base64::engine::general_purpose::URL_SAFE_NO_PAD;
use common::{fake_player, get, state_without_db};
use oxistream_server::config::YouTubeConfig;
use oxistream_server::youtube::{self, PlayerSource, YouTube};
use oxistream_server::{AppState, app};
use rustypipe::model::VideoPlayer;
use serde_json::json;

const VIDEO_ID: &str = "dQw4w9WgXcQ";
const BARE_ID: &str = "jNQXAC9IVRw";
const SPRITE: &str = "https://i.ytimg.com/sb/dQw4w9WgXcQ/storyboard3_L2/M$M.jpg?sqp=x&sigh=y";

struct Fixture;

#[async_trait]
impl PlayerSource for Fixture {
    async fn player(&self, video_id: &str) -> youtube::Result<VideoPlayer> {
        let mut player = fake_player("https://rr1---sn-test.googlevideo.com/", video_id);
        if video_id == VIDEO_ID {
            player.preview_frames = serde_json::from_value(json!([{
                "url_template": SPRITE,
                "frame_width": 160,
                "frame_height": 90,
                "page_count": 5,
                "total_count": 107,
                "duration_per_frame": 2000,
                "frames_per_page_x": 5,
                "frames_per_page_y": 5,
            }]))
            .unwrap();
        }
        Ok(player)
    }
}

fn router() -> Router {
    app(AppState {
        youtube: YouTube::with_player_source(&YouTubeConfig::default(), Arc::new(Fixture)).unwrap(),
        ..state_without_db()
    })
}

#[tokio::test]
async fn serves_thumbnails_track_with_proxied_sprites() {
    let (status, vtt) = get(
        router(),
        &format!("/api/v0/videos/{VIDEO_ID}/storyboard.vtt"),
    )
    .await;
    assert_eq!(status, StatusCode::OK);
    assert!(vtt.starts_with("WEBVTT\n"), "{vtt}");
    let urls: Vec<&str> = vtt.lines().filter(|l| l.starts_with('/')).collect();
    assert_eq!(urls.len(), 107);
    for url in &urls {
        assert!(url.starts_with("/proxy/image/"), "{url}");
        assert!(url.contains("&sig=") && url.contains("#xywh="), "{url}");
    }
    assert!(urls[26].ends_with("#xywh=160,0,160,90"), "{}", urls[26]);

    let encoded = urls[26]
        .trim_start_matches("/proxy/image/")
        .split('?')
        .next()
        .unwrap();
    let upstream = String::from_utf8(URL_SAFE_NO_PAD.decode(encoded).unwrap()).unwrap();
    assert_eq!(upstream, SPRITE.replace("$M", "1"));
}

#[tokio::test]
async fn missing_storyboard_is_not_found() {
    let (status, body) = get(
        router(),
        &format!("/api/v0/videos/{BARE_ID}/storyboard.vtt"),
    )
    .await;
    assert_eq!(status, StatusCode::NOT_FOUND);
    let json: serde_json::Value = serde_json::from_str(&body).unwrap();
    assert_eq!(json["reason"], "no_storyboard");

    let (status, _) = get(router(), "/api/v0/videos/nope/storyboard.vtt").await;
    assert_eq!(status, StatusCode::BAD_REQUEST);
}
