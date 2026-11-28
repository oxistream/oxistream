use std::fmt::Write as _;
use std::time::Duration;

use axum::extract::{Path, State};
use axum::http::{HeaderValue, StatusCode, header};
use axum::response::{IntoResponse, Response};
use rustypipe::model as rp;
use utoipa_axum::router::OpenApiRouter;
use utoipa_axum::routes;

use super::manifest::duration;
use crate::AppState;
use crate::api::error::{ApiError, ErrorBody, ErrorCode};

pub fn router() -> OpenApiRouter<AppState> {
    OpenApiRouter::new().routes(routes!(storyboard))
}

/// Path of the storyboard track of a video that has one.
pub fn track_url(player: &rp::VideoPlayer) -> Option<String> {
    (!player.details.is_live && best_frameset(&player.preview_frames).is_some())
        .then(|| format!("/api/v0/videos/{}/storyboard.vtt", player.details.id))
}

/// Get storyboard
///
/// Seek preview thumbnails as a `WebVTT` track, with cues pointing into proxied sprite sheets.
#[utoipa::path(
    get,
    path = "/videos/{id}/storyboard.vtt",
    tag = "youtube",
    params(("id" = String, Path, description = "YouTube video ID")),
    responses(
        (status = OK, description = "WebVTT thumbnails track", content_type = "text/vtt", body = String),
        (status = BAD_REQUEST, description = "Invalid video ID", body = ErrorBody),
        (status = NOT_FOUND, description = "Not found, unavailable or without storyboard", body = ErrorBody),
        (status = BAD_GATEWAY, description = "YouTube request failed", body = ErrorBody),
        (status = SERVICE_UNAVAILABLE, description = "Rate limited by YouTube", body = ErrorBody),
    )
)]
async fn storyboard(
    State(state): State<AppState>,
    Path(id): Path<String>,
) -> Result<Response, ApiError> {
    let player = state.youtube.player(&id).await?;
    let vtt = track_url(&player)
        .and_then(|_| best_frameset(&player.preview_frames))
        .and_then(|frameset| vtt(frameset, duration(&player), &|url| state.image_url(url)))
        .ok_or_else(|| {
            let mut err = ApiError::new(
                StatusCode::NOT_FOUND,
                ErrorCode::Unavailable,
                "the video has no storyboard",
            );
            err.body.reason = Some("no_storyboard".to_owned());
            err
        })?;
    Ok((
        [
            (
                header::CONTENT_TYPE,
                HeaderValue::from_static("text/vtt; charset=utf-8"),
            ),
            // Sprite URLs are signed for at least a day.
            (
                header::CACHE_CONTROL,
                HeaderValue::from_static("private, max-age=3600"),
            ),
        ],
        vtt,
    )
        .into_response())
}

/// The level with the largest frames.
fn best_frameset(framesets: &[rp::Frameset]) -> Option<&rp::Frameset> {
    framesets
        .iter()
        .filter(|f| {
            f.total_count > 0
                && f.frame_width > 0
                && f.frame_height > 0
                && f.frames_per_page_x > 0
                && f.frames_per_page_y > 0
                && f.url_template.contains("$M")
        })
        .max_by_key(|f| (f.frame_width * f.frame_height, f.total_count))
}

fn vtt(
    frameset: &rp::Frameset,
    duration: Duration,
    image_url: &impl Fn(&str) -> String,
) -> Option<String> {
    let total = u64::from(frameset.total_count);
    let duration_ms = u64::try_from(duration.as_millis()).ok()?;
    // The lowest level spreads its frames evenly and has no interval of its own.
    let interval_ms = match u64::from(frameset.duration_per_frame) {
        0 => duration_ms.div_ceil(total),
        ms => ms,
    };
    if interval_ms == 0 {
        return None;
    }
    let per_page = frameset.frames_per_page_x * frameset.frames_per_page_y;
    let pages: Vec<String> = (0..frameset.total_count.div_ceil(per_page))
        .map(|page| image_url(&frameset.url_template.replace("$M", &page.to_string())))
        .collect();

    let mut out = String::from("WEBVTT\n");
    for i in 0..frameset.total_count {
        let start = u64::from(i) * interval_ms;
        if start >= duration_ms {
            break;
        }
        let end = if i + 1 == frameset.total_count {
            duration_ms
        } else {
            (start + interval_ms).min(duration_ms)
        };
        let index = i % per_page;
        let x = index % frameset.frames_per_page_x * frameset.frame_width;
        let y = index / frameset.frames_per_page_x * frameset.frame_height;
        let _ = write!(
            out,
            "\n{} --> {}\n{}#xywh={x},{y},{},{}\n",
            timestamp(start),
            timestamp(end),
            pages[(i / per_page) as usize],
            frameset.frame_width,
            frameset.frame_height,
        );
    }
    Some(out)
}

fn timestamp(ms: u64) -> String {
    format!(
        "{:02}:{:02}:{:02}.{:03}",
        ms / 3_600_000,
        ms / 60_000 % 60,
        ms / 1000 % 60,
        ms % 1000
    )
}

#[cfg(test)]
mod tests {
    use serde_json::json;

    use super::*;

    fn frameset(width: u32, count: u32, per_frame: u32, grid: u32) -> rp::Frameset {
        serde_json::from_value(json!({
            "url_template": format!("https://i.ytimg.com/sb/x/storyboard3_L{width}/M$M.jpg?sqp=a&sigh=b"),
            "frame_width": width,
            "frame_height": width * 9 / 16,
            "page_count": count.div_ceil(grid * grid),
            "total_count": count,
            "duration_per_frame": per_frame,
            "frames_per_page_x": grid,
            "frames_per_page_y": grid,
        }))
        .unwrap()
    }

    fn img(url: &str) -> String {
        format!("/proxy/image/{}", url.rsplit('/').next().unwrap())
    }

    #[test]
    fn picks_the_largest_frames() {
        let sets = [
            frameset(48, 100, 0, 10),
            frameset(160, 107, 2000, 5),
            frameset(80, 107, 2000, 10),
        ];
        assert_eq!(best_frameset(&sets).unwrap().frame_width, 160);
        assert!(best_frameset(&[]).is_none());
        let mut broken = frameset(320, 10, 2000, 3);
        broken.frames_per_page_x = 0;
        assert_eq!(
            best_frameset(&[broken, frameset(48, 100, 0, 10)])
                .unwrap()
                .frame_width,
            48
        );
    }

    #[test]
    fn builds_cues_across_sprite_pages() {
        let vtt = vtt(
            &frameset(160, 11, 2000, 3),
            Duration::from_millis(21_500),
            &img,
        )
        .unwrap();
        insta::assert_snapshot!(vtt);
    }

    #[test]
    fn spreads_frames_without_interval() {
        let vtt = vtt(&frameset(48, 4, 0, 10), Duration::from_secs(10), &img).unwrap();
        assert!(vtt.contains("00:00:00.000 --> 00:00:02.500\n"), "{vtt}");
        assert!(
            vtt.contains(
                "00:00:07.500 --> 00:00:10.000\n/proxy/image/M0.jpg?sqp=a&sigh=b#xywh=144,0,48,27"
            ),
            "{vtt}"
        );
        assert!(super::vtt(&frameset(48, 4, 0, 10), Duration::ZERO, &img).is_none());
    }

    #[test]
    fn formats_timestamps() {
        assert_eq!(timestamp(3_723_004), "01:02:03.004");
    }
}
