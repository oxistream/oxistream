use std::collections::HashSet;
use std::time::Duration;

use axum::extract::{Path, State};
use axum::http::{HeaderValue, StatusCode, header};
use axum::response::{IntoResponse, Response};
use oxistream_dash as dash;
use rustypipe::model as rp;
use utoipa_axum::router::OpenApiRouter;
use utoipa_axum::routes;

use crate::AppState;
use crate::api::error::{ApiError, ErrorBody, ErrorCode};

pub fn router() -> OpenApiRouter<AppState> {
    OpenApiRouter::new().routes(routes!(manifest))
}

/// DASH manifest of a video, with all stream URLs pointing to the proxy.
#[utoipa::path(
    get,
    path = "/videos/{id}/manifest.mpd",
    tag = "youtube",
    params(("id" = String, Path, description = "YouTube video ID")),
    responses(
        (status = OK, description = "Static MPD", content_type = "application/dash+xml", body = String),
        (status = BAD_REQUEST, description = "Invalid video ID", body = ErrorBody),
        (status = NOT_FOUND, description = "Not found, unavailable, live or without playable streams", body = ErrorBody),
        (status = BAD_GATEWAY, description = "YouTube request failed", body = ErrorBody),
        (status = SERVICE_UNAVAILABLE, description = "Rate limited by YouTube", body = ErrorBody),
    )
)]
async fn manifest(
    State(state): State<AppState>,
    Path(id): Path<String>,
) -> Result<Response, ApiError> {
    let player = state.youtube.player(&id).await?;
    if player.details.is_live {
        return Err(unavailable(
            "livestream",
            "livestreams are not supported yet",
        ));
    }
    let (video, audio) = dash_streams(&player, &|itag, track| state.stream_url(&id, itag, track));
    let mpd = dash::generate(duration(&player), &video, &audio).map_err(|err| match err {
        dash::Error::NoPlayableStreams => {
            unavailable("no_playable_streams", "the video has no playable streams")
        }
        dash::Error::Io(err) => {
            tracing::error!(error = %err, "could not write MPD");
            ApiError::new(
                StatusCode::INTERNAL_SERVER_ERROR,
                ErrorCode::UpstreamError,
                "could not generate manifest",
            )
        }
    })?;
    Ok((
        [
            (
                header::CONTENT_TYPE,
                HeaderValue::from_static("application/dash+xml"),
            ),
            // Stream URLs are signed for hours; a short TTL keeps reloads cheap.
            (
                header::CACHE_CONTROL,
                HeaderValue::from_static("private, max-age=60"),
            ),
        ],
        mpd,
    )
        .into_response())
}

fn unavailable(reason: &str, message: &str) -> ApiError {
    let mut err = ApiError::new(StatusCode::NOT_FOUND, ErrorCode::Unavailable, message);
    err.body.reason = Some(reason.to_owned());
    err
}

pub(super) fn duration(player: &rp::VideoPlayer) -> Duration {
    let stream_ms = player
        .video_only_streams
        .iter()
        .filter_map(|s| s.duration_ms)
        .chain(player.audio_streams.iter().filter_map(|s| s.duration_ms))
        .max();
    stream_ms.map_or_else(
        || Duration::from_secs(player.details.duration.into()),
        |ms| Duration::from_millis(ms.into()),
    )
}

/// Maps adaptive streams to MPD input. Only the first stream per itag and audio track is kept,
/// since that is the one the proxy resolves.
pub fn dash_streams(
    player: &rp::VideoPlayer,
    url: &impl Fn(u32, Option<&str>) -> String,
) -> (Vec<dash::VideoStream>, Vec<dash::AudioStream>) {
    let mut seen_video = HashSet::new();
    let video = player
        .video_only_streams
        .iter()
        .filter(|s| seen_video.insert(s.itag))
        .map(|s| dash::VideoStream {
            url: url(s.itag, None),
            itag: s.itag,
            bitrate: s.bitrate,
            init_range: s.init_range.clone(),
            index_range: s.index_range.clone(),
            width: s.width,
            height: s.height,
            fps: s.fps,
            hdr: s.hdr,
            mime: s.mime.clone(),
            drm: s.drm_track_type.is_some(),
        })
        .collect();
    let mut seen_audio = HashSet::new();
    let audio = player
        .audio_streams
        .iter()
        .filter(|s| seen_audio.insert((s.itag, s.track.as_ref().map(|t| t.id.as_str()))))
        .map(|s| {
            let track_id = s.track.as_ref().map(|t| t.id.as_str());
            dash::AudioStream {
                url: url(s.itag, track_id),
                itag: s.itag,
                bitrate: s.bitrate,
                init_range: s.init_range.clone(),
                index_range: s.index_range.clone(),
                mime: s.mime.clone(),
                channels: s.channels,
                // RustyPipe does not expose the sampling rate.
                sample_rate: None,
                track: s.track.as_ref().map(audio_track),
                drm: s.drm_track_type.is_some(),
            }
        })
        .collect();
    (video, audio)
}

fn audio_track(track: &rp::AudioTrack) -> dash::AudioTrack {
    dash::AudioTrack {
        id: track.id.clone(),
        lang: track.lang.clone(),
        lang_name: track.lang_name.clone(),
        is_default: track.is_default,
        track_type: track.track_type.and_then(|t| match t {
            rp::AudioTrackType::Original => Some(dash::AudioTrackType::Original),
            rp::AudioTrackType::Dubbed => Some(dash::AudioTrackType::Dubbed),
            rp::AudioTrackType::DubbedAuto => Some(dash::AudioTrackType::DubbedAuto),
            rp::AudioTrackType::Descriptive => Some(dash::AudioTrackType::Descriptive),
            _ => None,
        }),
    }
}

#[cfg(test)]
mod tests {
    use serde_json::{Value, json};

    use super::*;

    fn stream(itag: u32, extra: &Value) -> Value {
        let mut value = json!({
            "url": format!("https://rr1.googlevideo.com/videoplayback?itag={itag}"),
            "itag": itag,
            "bitrate": 1000,
            "average_bitrate": 900,
            "size": 10_000,
            "index_range": { "start": 741, "end": 1204 },
            "init_range": { "start": 0, "end": 740 },
            "duration_ms": 212_091,
            "drm_track_type": null,
            "drm_systems": [],
        });
        value
            .as_object_mut()
            .unwrap()
            .extend(extra.as_object().unwrap().clone());
        value
    }

    fn video(itag: u32, mime: &str, hdr: bool) -> Value {
        stream(
            itag,
            &json!({
                "width": 1920, "height": 1080, "fps": 30, "quality": "1080p", "hdr": hdr,
                "mime": mime, "format": "webm", "codec": "vp9",
            }),
        )
    }

    fn audio(itag: u32, track: Option<(&str, bool, &str)>) -> Value {
        stream(
            itag,
            &json!({
                "mime": "audio/webm; codecs=\"opus\"", "format": "webm", "codec": "opus",
                "channels": 2, "loudness_db": null,
                "track": track.map(|(id, is_default, kind)| json!({
                    "id": id, "lang": null, "lang_name": id, "is_default": is_default,
                    "track_type": kind,
                })),
            }),
        )
    }

    fn player(video: &[Value], audio: &[Value], is_live: bool) -> rp::VideoPlayer {
        serde_json::from_value(json!({
            "details": {
                "id": "dQw4w9WgXcQ", "name": null, "description": null, "duration": 213,
                "thumbnail": [], "channel_id": "UCuAXFkgsw1L7xaCfnd5JJOw", "channel_name": null,
                "view_count": null, "keywords": [], "is_live": is_live, "is_live_content": is_live,
            },
            "video_streams": [],
            "video_only_streams": video,
            "audio_streams": audio,
            "subtitles": [],
            "expires_in_seconds": 21_540,
            "valid_until": "2099-01-01T00:00:00Z",
            "hls_manifest_url": null,
            "dash_manifest_url": null,
            "preview_frames": [],
            "drm": null,
            "client_type": "desktop",
            "visitor_data": null,
        }))
        .unwrap()
    }

    fn url(itag: u32, track: Option<&str>) -> String {
        format!("/proxy/stream/x/{itag}?track={}", track.unwrap_or_default())
    }

    #[test]
    fn maps_streams_with_proxied_urls() {
        let mut drm = video(399, "video/mp4; codecs=\"av01.0.08M.08\"", false);
        drm["drm_track_type"] = json!("hd");
        let p = player(
            &[
                video(
                    337,
                    "video/webm; codecs=\"vp09.02.51.10.01.09.16.09.00\"",
                    true,
                ),
                drm,
            ],
            &[
                audio(251, Some(("en.4", true, "original"))),
                audio(251, Some(("de.3", false, "dubbed-auto"))),
            ],
            false,
        );
        let (video, audio) = dash_streams(&p, &url);

        assert_eq!(video.len(), 2);
        assert_eq!(video[0].url, "/proxy/stream/x/337?track=");
        assert_eq!(
            video[0].mime,
            "video/webm; codecs=\"vp09.02.51.10.01.09.16.09.00\""
        );
        assert_eq!(video[0].init_range, Some(0..740));
        assert!(video[0].hdr && !video[0].drm);
        assert!(video[1].drm);

        assert_eq!(audio.len(), 2);
        assert_eq!(audio[1].url, "/proxy/stream/x/251?track=de.3");
        assert_eq!(audio[1].sample_rate, None);
        assert_eq!(audio[1].channels, Some(2));
        let track = audio[1].track.as_ref().unwrap();
        assert_eq!(track.id, "de.3");
        assert!(!track.is_default);
        assert_eq!(track.track_type, Some(dash::AudioTrackType::DubbedAuto));
        assert!(audio[0].track.as_ref().unwrap().is_default);
    }

    #[test]
    fn keeps_first_stream_per_itag_and_track() {
        let mut drc = audio(251, None);
        drc["url"] = json!("https://rr1.googlevideo.com/videoplayback?itag=251&xtags=drc%3D1");
        let p = player(&[], &[audio(251, None), drc, audio(140, None)], false);
        let (_, audio) = dash_streams(&p, &url);
        assert_eq!(audio.iter().map(|a| a.itag).collect::<Vec<_>>(), [251, 140]);
    }

    #[test]
    fn duration_prefers_stream_milliseconds() {
        let p = player(
            &[video(137, "video/mp4; codecs=\"avc1.640028\"", false)],
            &[],
            false,
        );
        assert_eq!(duration(&p), Duration::from_millis(212_091));
        let p = player(&[], &[], false);
        assert_eq!(duration(&p), Duration::from_secs(213));
    }
}
