//! Proxy for streams, images and subtitles behind stateless HMAC-signed URLs.
//!
//! Stream and subtitle URLs only name the video and format; each replica resolves them against
//! its own player cache, so upstream URLs are always used from the egress IP that fetched them.

use std::collections::HashMap;
use std::fmt::Write as _;
use std::sync::Arc;
use std::time::{Duration, SystemTime, UNIX_EPOCH};

use axum::Router;
use axum::body::Body;
use axum::extract::{Path, Query, State};
use axum::http::{HeaderMap, HeaderValue, StatusCode, header};
use axum::response::{IntoResponse, Response};
use axum::routing::get;
use base64::Engine;
use base64::engine::general_purpose::URL_SAFE_NO_PAD;
use futures_util::TryStreamExt;
use reqwest::Url;
use rustypipe::client::ClientType;
use rustypipe::model::{Subtitle, VideoPlayer};
use serde::Deserialize;

use crate::AppState;
use crate::api::error::ApiError;
use crate::config::ProxyConfig;
use crate::youtube::YouTubeError;

mod allowlist;
mod sign;

pub use allowlist::Allowlist;
pub use sign::{Kind, SignError, Signer, expiry};

const STREAM_BUCKET: Duration = Duration::from_mins(10);
const IMAGE_BUCKET: Duration = Duration::from_hours(24);
const CONNECT_TIMEOUT: Duration = Duration::from_secs(10);
const READ_TIMEOUT: Duration = Duration::from_secs(30);
const MAX_REDIRECTS: usize = 5;
/// Upper bound for waiting on `VideoPlayer::available_at`; pre-roll ads are rarely longer.
const MAX_AD_WAIT: Duration = Duration::from_secs(30);

const PASSTHROUGH_HEADERS: [header::HeaderName; 4] = [
    header::CONTENT_TYPE,
    header::CONTENT_LENGTH,
    header::CONTENT_RANGE,
    header::ACCEPT_RANGES,
];

#[derive(Clone)]
pub struct Proxy {
    inner: Arc<Inner>,
}

struct Inner {
    signer: Signer,
    allowlist: Allowlist,
    http: reqwest::Client,
    stream_ttl: Duration,
    image_ttl: Duration,
}

impl Proxy {
    pub fn new(config: &ProxyConfig) -> anyhow::Result<Self> {
        Self::with_allowlist(config, Allowlist::youtube())
    }

    pub fn with_allowlist(config: &ProxyConfig, allowlist: Allowlist) -> anyhow::Result<Self> {
        let redirect_allowlist = allowlist.clone();
        let http = reqwest::Client::builder()
            .tls_backend_preconfigured(tls_config()?)
            .connect_timeout(CONNECT_TIMEOUT)
            .read_timeout(READ_TIMEOUT)
            .redirect(reqwest::redirect::Policy::custom(move |attempt| {
                if attempt.previous().len() >= MAX_REDIRECTS {
                    attempt.error("too many redirects")
                } else if redirect_allowlist.allows_any(attempt.url()) {
                    attempt.follow()
                } else {
                    attempt.stop()
                }
            }))
            .build()?;
        Ok(Self {
            inner: Arc::new(Inner {
                signer: Signer::new(config.secret.as_bytes()),
                allowlist,
                http,
                stream_ttl: config.stream_ttl,
                image_ttl: config.image_ttl,
            }),
        })
    }

    fn stream_exp(&self) -> u64 {
        expiry(now(), self.inner.stream_ttl, STREAM_BUCKET)
    }

    fn query(&self, kind: Kind, payload: &str, exp: u64) -> String {
        format!(
            "exp={exp}&sig={}",
            self.inner.signer.sign(kind, payload, exp)
        )
    }

    pub fn image_url(&self, upstream: &str) -> String {
        let upstream = absolute(upstream);
        let encoded = URL_SAFE_NO_PAD.encode(upstream.as_bytes());
        let exp = expiry(now(), self.inner.image_ttl, IMAGE_BUCKET);
        format!(
            "/proxy/image/{encoded}?{}",
            self.query(Kind::Image, &encoded, exp)
        )
    }

    pub fn stream_url(&self, video_id: &str, itag: u32, track: Option<&str>) -> String {
        let track = track.unwrap_or_default();
        let query = self.query(
            Kind::Stream,
            &stream_payload(video_id, itag, track),
            self.stream_exp(),
        );
        if track.is_empty() {
            format!("/proxy/stream/{video_id}/{itag}?{query}")
        } else {
            format!(
                "/proxy/stream/{video_id}/{itag}?track={}&{query}",
                encode(track)
            )
        }
    }

    pub fn subtitle_url(&self, video_id: &str, id: &str) -> String {
        let query = self.query(
            Kind::Subtitle,
            &subtitle_payload(video_id, id),
            self.stream_exp(),
        );
        format!("/proxy/subtitle/{video_id}/{}?{query}", encode(id))
    }

    fn verify(&self, kind: Kind, payload: &str, exp: u64, sig: &str) -> Result<(), ProxyError> {
        self.inner
            .signer
            .verify(kind, payload, exp, sig, now())
            .map_err(ProxyError::Signature)
    }

    async fn fetch(
        &self,
        kind: Kind,
        url: &str,
        user_agent: &str,
        range: Option<&HeaderValue>,
    ) -> Result<reqwest::Response, ProxyError> {
        let url = Url::parse(url).map_err(|_| ProxyError::Forbidden)?;
        if !self.inner.allowlist.allows(kind, &url) {
            return Err(ProxyError::Forbidden);
        }
        let mut request = self
            .inner
            .http
            .get(url)
            .header(header::USER_AGENT, user_agent);
        if let Some(range) = range {
            request = request.header(header::RANGE, range);
        }
        let response = request.send().await.map_err(|err| {
            metrics::counter!("proxy_upstream_responses_total", "kind" => kind.as_str(), "status" => "error")
                .increment(1);
            tracing::warn!(kind = kind.as_str(), error = %err, "proxy upstream request failed");
            ProxyError::Upstream
        })?;
        metrics::counter!(
            "proxy_upstream_responses_total",
            "kind" => kind.as_str(),
            "status" => response.status().as_u16().to_string()
        )
        .increment(1);
        if !self.inner.allowlist.allows(kind, response.url()) {
            return Err(ProxyError::Forbidden);
        }
        Ok(response)
    }
}

pub(crate) fn tls_config() -> anyhow::Result<rustls::ClientConfig> {
    let roots = rustls::RootCertStore {
        roots: webpki_roots::TLS_SERVER_ROOTS.to_vec(),
    };
    Ok(rustls::ClientConfig::builder_with_provider(Arc::new(
        rustls::crypto::ring::default_provider(),
    ))
    .with_safe_default_protocol_versions()?
    .with_root_certificates(roots)
    .with_no_client_auth())
}

fn now() -> u64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map_or(0, |d| d.as_secs())
}

/// YouTube sometimes hands out protocol-relative image URLs.
fn absolute(url: &str) -> std::borrow::Cow<'_, str> {
    match url.strip_prefix("//") {
        Some(rest) => format!("https://{rest}").into(),
        None => url.into(),
    }
}

fn stream_payload(video_id: &str, itag: u32, track: &str) -> String {
    format!("{video_id}/{itag}/{track}")
}

fn subtitle_payload(video_id: &str, id: &str) -> String {
    format!("{video_id}/{id}")
}

/// Percent-encodes everything but RFC 3986 unreserved characters.
fn encode(value: &str) -> String {
    let mut out = String::with_capacity(value.len());
    for byte in value.bytes() {
        if byte.is_ascii_alphanumeric() || matches!(byte, b'-' | b'.' | b'_' | b'~') {
            out.push(char::from(byte));
        } else {
            let _ = write!(out, "%{byte:02X}");
        }
    }
    out
}

/// Subtitle IDs as used in proxy URLs, in track order: the language code, prefixed with `a.` if
/// auto-generated. Later tracks with the same code get a `.2`, `.3`… suffix (YouTube can list
/// several tracks per language, e.g. differently named manual captions).
fn unique_subtitle_ids<'a>(tracks: impl IntoIterator<Item = (&'a str, bool)>) -> Vec<String> {
    let mut seen = HashMap::<String, usize>::new();
    tracks
        .into_iter()
        .map(|(lang, auto_generated)| {
            let base = if auto_generated {
                format!("a.{lang}")
            } else {
                lang.to_owned()
            };
            let count = seen.entry(base.clone()).or_default();
            *count += 1;
            if *count == 1 {
                base
            } else {
                format!("{base}.{count}")
            }
        })
        .collect()
}

pub fn subtitle_ids(subtitles: &[Subtitle]) -> Vec<String> {
    unique_subtitle_ids(
        subtitles
            .iter()
            .map(|s| (s.lang.as_str(), s.auto_generated)),
    )
}

#[derive(Debug)]
enum ProxyError {
    Signature(SignError),
    Forbidden,
    NotFound,
    Upstream,
    YouTube(YouTubeError),
}

impl From<YouTubeError> for ProxyError {
    fn from(err: YouTubeError) -> Self {
        Self::YouTube(err)
    }
}

impl IntoResponse for ProxyError {
    fn into_response(self) -> Response {
        let (status, message) = match self {
            Self::Signature(SignError::Invalid) => (StatusCode::FORBIDDEN, "invalid signature"),
            Self::Signature(SignError::Expired) => (StatusCode::FORBIDDEN, "link expired"),
            Self::Forbidden => (StatusCode::FORBIDDEN, "upstream not allowed"),
            Self::NotFound => (StatusCode::NOT_FOUND, "not found"),
            Self::Upstream => (StatusCode::BAD_GATEWAY, "upstream request failed"),
            Self::YouTube(err) => {
                let mut response = ApiError::from(err).into_response();
                response
                    .headers_mut()
                    .insert(header::CACHE_CONTROL, HeaderValue::from_static("no-store"));
                return response;
            }
        };
        (
            status,
            [(header::CACHE_CONTROL, HeaderValue::from_static("no-store"))],
            message,
        )
            .into_response()
    }
}

#[derive(Debug, Deserialize)]
struct Signed {
    exp: u64,
    sig: String,
}

#[derive(Debug, Deserialize)]
struct StreamQuery {
    #[serde(default)]
    track: String,
    exp: u64,
    sig: String,
}

pub fn router() -> Router<AppState> {
    Router::new()
        .route("/proxy/stream/{video_id}/{itag}", get(stream))
        .route("/proxy/subtitle/{video_id}/{id}", get(subtitle))
        .route("/proxy/image/{encoded}", get(image))
}

/// Proxies a stream, refreshing the player data once if YouTube rejects the cached URL.
async fn stream(
    State(state): State<AppState>,
    Path((video_id, itag)): Path<(String, u32)>,
    Query(query): Query<StreamQuery>,
    headers: HeaderMap,
) -> Result<Response, ProxyError> {
    let proxy = &state.proxy;
    proxy.verify(
        Kind::Stream,
        &stream_payload(&video_id, itag, &query.track),
        query.exp,
        &query.sig,
    )?;
    let range = headers.get(header::RANGE);
    let response = fetch_with_refresh(&state, &video_id, Kind::Stream, range, |player| {
        find_stream(player, itag, &query.track).map(str::to_owned)
    })
    .await?;
    passthrough(Kind::Stream, response)
}

async fn subtitle(
    State(state): State<AppState>,
    Path((video_id, id)): Path<(String, String)>,
    Query(signed): Query<Signed>,
) -> Result<Response, ProxyError> {
    state.proxy.verify(
        Kind::Subtitle,
        &subtitle_payload(&video_id, &id),
        signed.exp,
        &signed.sig,
    )?;
    let response = fetch_with_refresh(&state, &video_id, Kind::Subtitle, None, |player| {
        subtitle_ids(&player.subtitles)
            .iter()
            .position(|candidate| *candidate == id)
            .and_then(|i| vtt_url(&player.subtitles[i].url))
    })
    .await?;
    let mut response = passthrough(Kind::Subtitle, response)?;
    if response.status().is_success() {
        response.headers_mut().insert(
            header::CONTENT_TYPE,
            HeaderValue::from_static("text/vtt; charset=utf-8"),
        );
    }
    Ok(response)
}

async fn image(
    State(state): State<AppState>,
    Path(encoded): Path<String>,
    Query(signed): Query<Signed>,
) -> Result<Response, ProxyError> {
    let proxy = &state.proxy;
    proxy.verify(Kind::Image, &encoded, signed.exp, &signed.sig)?;
    let url = URL_SAFE_NO_PAD
        .decode(&encoded)
        .ok()
        .and_then(|bytes| String::from_utf8(bytes).ok())
        .ok_or(ProxyError::Forbidden)?;
    let user_agent = state.youtube.user_agent(ClientType::Desktop);
    let response = proxy.fetch(Kind::Image, &url, &user_agent, None).await?;
    passthrough(Kind::Image, response)
}

async fn fetch_with_refresh(
    state: &AppState,
    video_id: &str,
    kind: Kind,
    range: Option<&HeaderValue>,
    resolve: impl Fn(&VideoPlayer) -> Option<String>,
) -> Result<reqwest::Response, ProxyError> {
    let mut refreshed = false;
    loop {
        let player = state.youtube.player(video_id).await?;
        wait_until_available(&player).await;
        let url = resolve(&player).ok_or(ProxyError::NotFound)?;
        let user_agent = state.youtube.user_agent(player.client_type);
        let response = state.proxy.fetch(kind, &url, &user_agent, range).await?;
        if response.status() == StatusCode::FORBIDDEN && !refreshed {
            tracing::info!(%video_id, "upstream rejected stream URL, refreshing player");
            state.youtube.invalidate(video_id).await;
            refreshed = true;
            continue;
        }
        return Ok(response);
    }
}

/// Some clients return streams that googlevideo rejects until a pre-roll ad would have ended.
async fn wait_until_available(player: &VideoPlayer) {
    let Some(at) = player.available_at else {
        return;
    };
    let remaining = at - time::OffsetDateTime::now_utc();
    if remaining.is_positive() {
        let remaining = Duration::try_from(remaining).unwrap_or_default();
        tokio::time::sleep(remaining.min(MAX_AD_WAIT)).await;
    }
}

fn find_stream<'a>(player: &'a VideoPlayer, itag: u32, track: &str) -> Option<&'a str> {
    let video = player
        .video_only_streams
        .iter()
        .chain(&player.video_streams)
        .find(|s| s.itag == itag)
        .map(|s| s.url.as_str());
    video.or_else(|| {
        player
            .audio_streams
            .iter()
            .find(|s| s.itag == itag && s.track.as_ref().map_or("", |t| t.id.as_str()) == track)
            .map(|s| s.url.as_str())
    })
}

fn vtt_url(url: &str) -> Option<String> {
    let mut url = Url::parse(&absolute(url)).ok()?;
    let pairs: Vec<(String, String)> = url
        .query_pairs()
        .filter(|(key, _)| key != "fmt")
        .map(|(k, v)| (k.into_owned(), v.into_owned()))
        .collect();
    url.query_pairs_mut()
        .clear()
        .extend_pairs(pairs)
        .append_pair("fmt", "vtt");
    Some(url.into())
}

fn passthrough(kind: Kind, upstream: reqwest::Response) -> Result<Response, ProxyError> {
    let status = upstream.status();
    if status == StatusCode::NOT_FOUND || status == StatusCode::GONE {
        return Err(ProxyError::NotFound);
    }
    if !(status.is_success() || status == StatusCode::RANGE_NOT_SATISFIABLE) {
        tracing::warn!(kind = kind.as_str(), %status, "unexpected upstream status");
        return Err(ProxyError::Upstream);
    }

    let mut headers = HeaderMap::new();
    for name in PASSTHROUGH_HEADERS {
        if let Some(value) = upstream.headers().get(&name) {
            headers.insert(name, value.clone());
        }
    }
    let cache_control = match kind {
        Kind::Image => "public, max-age=86400",
        Kind::Stream | Kind::Subtitle => "private, max-age=3600",
    };
    headers.insert(
        header::CACHE_CONTROL,
        HeaderValue::from_static(cache_control),
    );
    headers.insert(
        header::X_CONTENT_TYPE_OPTIONS,
        HeaderValue::from_static("nosniff"),
    );

    let kind_label = kind.as_str();
    let body = upstream
        .bytes_stream()
        .inspect_ok(move |chunk| {
            metrics::counter!("proxy_bytes_total", "kind" => kind_label)
                .increment(chunk.len() as u64);
        })
        .inspect_err(move |err| {
            tracing::debug!(kind = kind_label, error = %err, "upstream body ended early");
        });

    let mut response = Response::new(Body::from_stream(body));
    *response.status_mut() = status;
    *response.headers_mut() = headers;
    Ok(response)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn encodes_query_values() {
        assert_eq!(encode("en.4"), "en.4");
        assert_eq!(encode("de-DE.3"), "de-DE.3");
        assert_eq!(encode("a b&c"), "a%20b%26c");
    }

    #[test]
    fn requests_vtt() {
        assert_eq!(
            vtt_url("https://www.youtube.com/api/timedtext?v=x&lang=en&fmt=srv3").unwrap(),
            "https://www.youtube.com/api/timedtext?v=x&lang=en&fmt=vtt"
        );
        assert_eq!(
            vtt_url("//www.youtube.com/api/timedtext?v=x").unwrap(),
            "https://www.youtube.com/api/timedtext?v=x&fmt=vtt"
        );
    }

    #[test]
    fn subtitle_ids_distinguish_auto_generated() {
        assert_eq!(
            unique_subtitle_ids([("en", false), ("en", true)]),
            ["en", "a.en"]
        );
    }

    #[test]
    fn subtitle_ids_are_unique_per_language() {
        assert_eq!(
            unique_subtitle_ids([
                ("en", false),
                ("es-419", false),
                ("en", false),
                ("en", true),
                ("en", false),
                ("en", true),
            ]),
            ["en", "es-419", "en.2", "a.en", "en.3", "a.en.2"]
        );
    }

    #[test]
    fn signed_urls_round_trip() {
        let proxy = Proxy::new(&ProxyConfig::new("0123456789abcdef0123456789abcdef")).unwrap();
        let url = Url::parse("http://x")
            .unwrap()
            .join(&proxy.stream_url("dQw4w9WgXcQ", 251, Some("en.4")))
            .unwrap();
        assert_eq!(url.path(), "/proxy/stream/dQw4w9WgXcQ/251");
        let query: StreamQuery = serde_urlencoded_query(&url);
        assert_eq!(query.track, "en.4");
        proxy
            .verify(
                Kind::Stream,
                &stream_payload("dQw4w9WgXcQ", 251, &query.track),
                query.exp,
                &query.sig,
            )
            .unwrap();

        let image = proxy.image_url("//i.ytimg.com/vi/x/hqdefault.jpg");
        let encoded = image
            .strip_prefix("/proxy/image/")
            .and_then(|rest| rest.split('?').next())
            .unwrap();
        assert_eq!(
            URL_SAFE_NO_PAD.decode(encoded).unwrap(),
            b"https://i.ytimg.com/vi/x/hqdefault.jpg"
        );
    }

    fn serde_urlencoded_query<T: serde::de::DeserializeOwned>(url: &Url) -> T {
        let uri: axum::http::Uri = url.as_str().parse().unwrap();
        Query::<T>::try_from_uri(&uri).unwrap().0
    }
}
