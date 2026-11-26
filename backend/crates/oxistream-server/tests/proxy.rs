//! Proxy handlers against a local mock upstream.

mod common;

use std::net::SocketAddr;
use std::num::NonZeroU32;
use std::sync::atomic::{AtomicUsize, Ordering};
use std::sync::{Arc, Mutex};
use std::time::{SystemTime, UNIX_EPOCH};

use async_trait::async_trait;
use axum::Router;
use axum::body::Body;
use axum::extract::{ConnectInfo, Query};
use axum::http::{HeaderMap, Request, StatusCode, Uri, header};
use axum::response::{IntoResponse, Redirect, Response};
use axum::routing::get;
use common::{TEST_PROXY_SECRET, fake_player, spawn_upstream, state_without_db};
use http_body_util::BodyExt;
use oxistream_server::config::{ProxyConfig, RateLimitConfig, YouTubeConfig};
use oxistream_server::proxy::{Allowlist, Kind, Proxy, Signer};
use oxistream_server::rate_limit::RateLimit;
use oxistream_server::youtube::{self, PlayerSource, YouTube};
use oxistream_server::{AppState, app};
use rustypipe::model::VideoPlayer;
use tower::ServiceExt;

const VIDEO_ID: &str = "dQw4w9WgXcQ";
const DATA: &[u8] = b"0123456789";

#[derive(Default)]
struct Seen {
    requests: Mutex<Vec<(Uri, HeaderMap)>>,
}

impl Seen {
    fn last(&self) -> (Uri, HeaderMap) {
        self.requests
            .lock()
            .unwrap()
            .last()
            .cloned()
            .expect("upstream was called")
    }
}

fn videoplayback(
    seen: &Seen,
    uri: Uri,
    headers: HeaderMap,
    Query(query): Query<Vec<(String, String)>>,
) -> Response {
    let range = headers
        .get(header::RANGE)
        .and_then(|v| v.to_str().ok())
        .and_then(|v| v.strip_prefix("bytes="))
        .and_then(|v| v.split_once('-'))
        .map(|(a, b)| (a.parse::<usize>().unwrap(), b.parse::<usize>().unwrap()));
    seen.requests.lock().unwrap().push((uri, headers));
    if query.iter().any(|(k, v)| k == "stale" && v == "1") {
        return StatusCode::FORBIDDEN.into_response();
    }
    let base = [
        (header::CONTENT_TYPE, "video/mp4"),
        (header::ACCEPT_RANGES, "bytes"),
        (header::SET_COOKIE, "tracking=1"),
    ];
    match range {
        None => (base, DATA).into_response(),
        Some((start, _)) if start >= DATA.len() => (
            StatusCode::RANGE_NOT_SATISFIABLE,
            [(header::CONTENT_RANGE, format!("bytes */{}", DATA.len()))],
        )
            .into_response(),
        Some((start, end)) => {
            let end = end.min(DATA.len() - 1);
            (
                StatusCode::PARTIAL_CONTENT,
                base,
                [(
                    header::CONTENT_RANGE,
                    format!("bytes {start}-{end}/{}", DATA.len()),
                )],
                DATA[start..=end].to_vec(),
            )
                .into_response()
        }
    }
}

fn upstream(seen: &Arc<Seen>) -> Router {
    let s1 = seen.clone();
    let s2 = seen.clone();
    let s3 = seen.clone();
    Router::new()
        .route(
            "/videoplayback",
            get(move |uri, headers, query| {
                std::future::ready(videoplayback(&s1, uri, headers, query))
            }),
        )
        .route(
            "/api/timedtext",
            get(move |uri: Uri, headers: HeaderMap| async move {
                s2.requests.lock().unwrap().push((uri, headers));
                ([(header::CONTENT_TYPE, "text/plain")], "WEBVTT\n\n")
            }),
        )
        .route(
            "/image.jpg",
            get(move |uri: Uri, headers: HeaderMap| async move {
                s3.requests.lock().unwrap().push((uri, headers));
                ([(header::CONTENT_TYPE, "image/jpeg")], "jpeg")
            }),
        )
        .route(
            "/redirect",
            get(|| async { Redirect::temporary("https://example.com/videoplayback") }),
        )
}

/// Serves the fixture player, with a stale stream URL for the first `stale` fetches.
struct Fixture {
    base: String,
    stale: usize,
    fetches: AtomicUsize,
}

#[async_trait]
impl PlayerSource for Fixture {
    async fn player(&self, video_id: &str) -> youtube::Result<VideoPlayer> {
        let n = self.fetches.fetch_add(1, Ordering::SeqCst);
        let mut player = fake_player(&self.base, video_id);
        if n < self.stale {
            for stream in &mut player.video_only_streams {
                stream.url.push_str("&stale=1");
            }
        }
        Ok(player)
    }
}

struct Harness {
    state: AppState,
    seen: Arc<Seen>,
    source: Arc<Fixture>,
    base: reqwest::Url,
}

impl Harness {
    async fn new(stale_fetches: usize) -> Self {
        let seen = Arc::new(Seen::default());
        let base = spawn_upstream(upstream(&seen)).await;
        let source = Arc::new(Fixture {
            base: base.to_string(),
            stale: stale_fetches,
            fetches: AtomicUsize::new(0),
        });
        let state = AppState {
            youtube: YouTube::with_player_source(&YouTubeConfig::default(), source.clone())
                .unwrap(),
            proxy: Proxy::with_allowlist(
                &ProxyConfig::new(TEST_PROXY_SECRET),
                Allowlist::youtube().allow_test_origin(&base),
            )
            .unwrap(),
            ..state_without_db()
        };
        Self {
            state,
            seen,
            source,
            base,
        }
    }

    async fn get(&self, uri: &str, headers: &[(header::HeaderName, &str)]) -> Response {
        let mut request = Request::get(uri);
        for (name, value) in headers {
            request = request.header(name, *value);
        }
        app(self.state.clone())
            .oneshot(request.body(Body::empty()).unwrap())
            .await
            .unwrap()
    }

    fn fetches(&self) -> usize {
        self.source.fetches.load(Ordering::SeqCst)
    }
}

async fn body(response: Response) -> Vec<u8> {
    response
        .into_body()
        .collect()
        .await
        .unwrap()
        .to_bytes()
        .to_vec()
}

fn now() -> u64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap()
        .as_secs()
}

#[tokio::test]
async fn streams_full_response() {
    let h = Harness::new(0).await;
    let response = h.get(&h.state.stream_url(VIDEO_ID, 137, None), &[]).await;
    assert_eq!(response.status(), StatusCode::OK);
    let headers = response.headers().clone();
    assert_eq!(headers[header::CONTENT_TYPE], "video/mp4");
    assert_eq!(headers[header::CONTENT_LENGTH], "10");
    assert_eq!(headers[header::ACCEPT_RANGES], "bytes");
    assert!(
        headers[header::CACHE_CONTROL]
            .to_str()
            .unwrap()
            .contains("max-age")
    );
    assert!(!headers.contains_key(header::SET_COOKIE));
    assert_eq!(body(response).await, DATA);
}

#[tokio::test]
async fn passes_ranges_through_without_client_identity() {
    let h = Harness::new(0).await;
    let response = h
        .get(
            &h.state.stream_url(VIDEO_ID, 137, None),
            &[
                (header::RANGE, "bytes=2-5"),
                (header::COOKIE, "session=secret"),
                (
                    header::HeaderName::from_static("x-forwarded-for"),
                    "203.0.113.9",
                ),
                (header::USER_AGENT, "client-browser"),
                (header::ACCEPT_ENCODING, "gzip"),
            ],
        )
        .await;
    assert_eq!(response.status(), StatusCode::PARTIAL_CONTENT);
    assert_eq!(response.headers()[header::CONTENT_RANGE], "bytes 2-5/10");
    assert_eq!(response.headers()[header::CONTENT_LENGTH], "4");
    assert!(!response.headers().contains_key(header::CONTENT_ENCODING));
    assert_eq!(body(response).await, b"2345");

    let (uri, headers) = h.seen.last();
    assert_eq!(uri.query(), Some("itag=137"));
    assert_eq!(headers[header::RANGE], "bytes=2-5");
    assert!(!headers.contains_key(header::COOKIE));
    assert!(!headers.contains_key("x-forwarded-for"));
    let ua = headers[header::USER_AGENT].to_str().unwrap();
    assert!(
        ua.starts_with("Mozilla/5.0") && ua != "client-browser",
        "{ua}"
    );
}

#[tokio::test]
async fn passes_unsatisfiable_range_through() {
    let h = Harness::new(0).await;
    let response = h
        .get(
            &h.state.stream_url(VIDEO_ID, 137, None),
            &[(header::RANGE, "bytes=100-200")],
        )
        .await;
    assert_eq!(response.status(), StatusCode::RANGE_NOT_SATISFIABLE);
    assert_eq!(response.headers()[header::CONTENT_RANGE], "bytes */10");
}

#[tokio::test]
async fn selects_audio_track() {
    let h = Harness::new(0).await;
    let response = h
        .get(&h.state.stream_url(VIDEO_ID, 251, Some("de.3")), &[])
        .await;
    assert_eq!(response.status(), StatusCode::OK);
    assert_eq!(h.seen.last().0.query(), Some("itag=251&track=de.3"));

    let response = h
        .get(&h.state.stream_url(VIDEO_ID, 251, Some("fr.1")), &[])
        .await;
    assert_eq!(response.status(), StatusCode::NOT_FOUND);
    let response = h.get(&h.state.stream_url(VIDEO_ID, 999, None), &[]).await;
    assert_eq!(response.status(), StatusCode::NOT_FOUND);
}

#[tokio::test]
async fn refreshes_player_once_on_upstream_403() {
    let h = Harness::new(1).await;
    let response = h.get(&h.state.stream_url(VIDEO_ID, 137, None), &[]).await;
    assert_eq!(response.status(), StatusCode::OK);
    assert_eq!(body(response).await, DATA);
    assert_eq!(h.fetches(), 2);
    assert_eq!(h.seen.requests.lock().unwrap().len(), 2);

    // The refreshed player is cached.
    let response = h.get(&h.state.stream_url(VIDEO_ID, 137, None), &[]).await;
    assert_eq!(response.status(), StatusCode::OK);
    assert_eq!(h.fetches(), 2);
}

#[tokio::test]
async fn gives_up_after_one_refresh() {
    let h = Harness::new(usize::MAX).await;
    let response = h.get(&h.state.stream_url(VIDEO_ID, 137, None), &[]).await;
    assert_eq!(response.status(), StatusCode::BAD_GATEWAY);
    assert_eq!(h.fetches(), 2);
    assert_eq!(h.seen.requests.lock().unwrap().len(), 2);
}

#[tokio::test]
async fn rejects_bad_signatures() {
    let h = Harness::new(0).await;
    let signer = Signer::new(TEST_PROXY_SECRET.as_bytes());
    let valid = h.state.stream_url(VIDEO_ID, 137, None);
    let past = now() - 1;
    let future = now() + 600;
    let expired_sig = signer.sign(Kind::Stream, &format!("{VIDEO_ID}/137/"), past);
    let image_sig = signer.sign(Kind::Image, &format!("{VIDEO_ID}/137/"), future);
    let cases = [
        valid.replace("/137?", "/251?"),
        valid.replace("sig=", "sig=x"),
        format!("/proxy/stream/{VIDEO_ID}/137?exp={past}&sig={expired_sig}"),
        format!("/proxy/stream/{VIDEO_ID}/137?exp={future}&sig={image_sig}"),
        format!("/proxy/stream/{VIDEO_ID}/137?exp={future}"),
        format!("{valid}&track=en.4"),
    ];
    for uri in cases {
        let status = h.get(&uri, &[]).await.status();
        assert!(
            status == StatusCode::FORBIDDEN || status == StatusCode::BAD_REQUEST,
            "{uri}: {status}"
        );
    }
    assert_eq!(h.seen.requests.lock().unwrap().len(), 0);
    assert_eq!(h.fetches(), 0);
}

#[tokio::test]
async fn proxies_subtitles_as_vtt() {
    let h = Harness::new(0).await;
    let response = h.get(&h.state.subtitle_url(VIDEO_ID, "a.en"), &[]).await;
    assert_eq!(response.status(), StatusCode::OK);
    assert_eq!(
        response.headers()[header::CONTENT_TYPE],
        "text/vtt; charset=utf-8"
    );
    assert_eq!(body(response).await, b"WEBVTT\n\n");
    let query = h.seen.last().0.query().unwrap().to_owned();
    assert_eq!(query, format!("v={VIDEO_ID}&lang=en&kind=asr&fmt=vtt"));

    let response = h.get(&h.state.subtitle_url(VIDEO_ID, "en"), &[]).await;
    assert_eq!(response.status(), StatusCode::OK);
    let query = h.seen.last().0.query().unwrap().to_owned();
    assert_eq!(query, format!("v={VIDEO_ID}&lang=en&fmt=vtt"));

    let response = h.get(&h.state.subtitle_url(VIDEO_ID, "fr"), &[]).await;
    assert_eq!(response.status(), StatusCode::NOT_FOUND);
}

#[tokio::test]
async fn proxies_allowlisted_images_only() {
    let h = Harness::new(0).await;
    let response = h
        .get(&h.state.image_url(&format!("{}image.jpg", h.base)), &[])
        .await;
    assert_eq!(response.status(), StatusCode::OK);
    assert_eq!(response.headers()[header::CONTENT_TYPE], "image/jpeg");
    assert!(
        response.headers()[header::CACHE_CONTROL]
            .to_str()
            .unwrap()
            .starts_with("public")
    );
    assert_eq!(body(response).await, b"jpeg");

    let requests = h.seen.requests.lock().unwrap().len();
    for url in [
        "https://example.com/image.jpg",
        "http://i.ytimg.com/vi/x/hq.jpg",
    ] {
        let response = h.get(&h.state.image_url(url), &[]).await;
        assert_eq!(response.status(), StatusCode::FORBIDDEN, "{url}");
    }
    assert_eq!(h.seen.requests.lock().unwrap().len(), requests);
}

#[tokio::test]
async fn does_not_follow_redirects_off_the_allowlist() {
    let h = Harness::new(0).await;
    let response = h
        .get(&h.state.image_url(&format!("{}redirect", h.base)), &[])
        .await;
    assert_eq!(response.status(), StatusCode::BAD_GATEWAY);
}

#[tokio::test]
async fn rate_limits_per_client_ip() {
    let rate_limit = RateLimit::new(&RateLimitConfig {
        per_second: NonZeroU32::new(1).unwrap(),
        burst: NonZeroU32::new(2).unwrap(),
        trusted_header: None,
    });
    let router = app(AppState {
        rate_limit: Some(rate_limit),
        ..state_without_db()
    });
    let request = |ip: &str| {
        let mut request = Request::get("/api/v0/instance")
            .body(Body::empty())
            .unwrap();
        request.extensions_mut().insert(ConnectInfo::<SocketAddr>(
            format!("{ip}:1234").parse().unwrap(),
        ));
        request
    };

    for _ in 0..2 {
        let response = router.clone().oneshot(request("192.0.2.1")).await.unwrap();
        assert_eq!(response.status(), StatusCode::OK);
    }
    let response = router.clone().oneshot(request("192.0.2.1")).await.unwrap();
    assert_eq!(response.status(), StatusCode::TOO_MANY_REQUESTS);
    assert!(response.headers().contains_key(header::RETRY_AFTER));

    let response = router.clone().oneshot(request("192.0.2.2")).await.unwrap();
    assert_eq!(response.status(), StatusCode::OK);

    // Operational endpoints are not limited.
    let mut healthz = Request::get("/healthz").body(Body::empty()).unwrap();
    healthz
        .extensions_mut()
        .insert(ConnectInfo::<SocketAddr>("192.0.2.1:1234".parse().unwrap()));
    assert_eq!(
        router.oneshot(healthz).await.unwrap().status(),
        StatusCode::OK
    );
}
