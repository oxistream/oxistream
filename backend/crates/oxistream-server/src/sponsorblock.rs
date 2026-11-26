//! SponsorBlock segments, fetched by SHA-256 hash prefix so the upstream server never learns
//! which video is watched, and never by clients themselves.

use std::fmt::Write as _;
use std::sync::Arc;
use std::time::{Duration, Instant};

use moka::Expiry;
use moka::future::Cache;
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use utoipa::ToSchema;

use crate::config::SponsorBlockConfig;

/// The shortest prefix the API accepts, and the one it recommends.
const PREFIX_LEN: usize = 4;
const CACHE_CAPACITY: u64 = 10_000;
const TTL: Duration = Duration::from_hours(1);
const EMPTY_TTL: Duration = Duration::from_mins(15);
const ERROR_TTL: Duration = Duration::from_mins(1);
const CONNECT_TIMEOUT: Duration = Duration::from_secs(3);
const REQUEST_TIMEOUT: Duration = Duration::from_secs(5);
const MAX_RESPONSE_BYTES: usize = 8 * 1024 * 1024;
const USER_AGENT: &str = concat!("Oxistream/", env!("CARGO_PKG_VERSION"));
const ACTION_TYPES: &str = r#"["skip","poi","full"]"#;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize, ToSchema)]
#[serde(rename_all = "snake_case")]
pub enum Category {
    Sponsor,
    Selfpromo,
    Interaction,
    Intro,
    Outro,
    Preview,
    Hook,
    MusicOfftopic,
    Filler,
    /// The part most viewers look for; a point in time.
    PoiHighlight,
    /// The whole video is about a product or service.
    ExclusiveAccess,
}

impl Category {
    pub const ALL: [Self; 11] = [
        Self::Sponsor,
        Self::Selfpromo,
        Self::Interaction,
        Self::Intro,
        Self::Outro,
        Self::Preview,
        Self::Hook,
        Self::MusicOfftopic,
        Self::Filler,
        Self::PoiHighlight,
        Self::ExclusiveAccess,
    ];

    pub fn as_str(self) -> &'static str {
        match self {
            Self::Sponsor => "sponsor",
            Self::Selfpromo => "selfpromo",
            Self::Interaction => "interaction",
            Self::Intro => "intro",
            Self::Outro => "outro",
            Self::Preview => "preview",
            Self::Hook => "hook",
            Self::MusicOfftopic => "music_offtopic",
            Self::Filler => "filler",
            Self::PoiHighlight => "poi_highlight",
            Self::ExclusiveAccess => "exclusive_access",
        }
    }

    fn parse(value: &str) -> Option<Self> {
        Self::ALL.into_iter().find(|c| c.as_str() == value)
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, ToSchema)]
#[serde(rename_all = "snake_case")]
pub enum Action {
    /// A range that can be skipped.
    Skip,
    /// A point in time (`start` equals `end`).
    Poi,
    /// Applies to the whole video.
    Full,
}

impl Action {
    fn parse(value: &str) -> Option<Self> {
        match value {
            "skip" => Some(Self::Skip),
            "poi" => Some(Self::Poi),
            "full" => Some(Self::Full),
            _ => None,
        }
    }
}

#[derive(Debug, Clone, PartialEq, Serialize, ToSchema)]
pub struct Segment {
    /// SponsorBlock UUID.
    pub id: String,
    pub category: Category,
    pub action: Action,
    /// Seconds.
    pub start: f64,
    /// Seconds.
    pub end: f64,
}

#[derive(Deserialize)]
struct UpstreamVideo {
    #[serde(rename = "videoID")]
    video_id: String,
    #[serde(default)]
    segments: Vec<UpstreamSegment>,
}

#[derive(Deserialize)]
struct UpstreamSegment {
    segment: Vec<f64>,
    #[serde(rename = "UUID")]
    uuid: String,
    category: String,
    #[serde(rename = "actionType")]
    action_type: String,
}

#[derive(Clone)]
struct Cached {
    segments: Arc<Vec<Segment>>,
    ttl: Duration,
}

struct CachedExpiry;

impl Expiry<String, Cached> for CachedExpiry {
    fn expire_after_create(
        &self,
        _key: &String,
        value: &Cached,
        _created_at: Instant,
    ) -> Option<Duration> {
        Some(value.ttl)
    }
}

#[derive(Clone)]
pub struct SponsorBlock {
    /// `None` when disabled.
    inner: Option<Arc<Inner>>,
}

struct Inner {
    http: reqwest::Client,
    api_url: String,
    cache: Cache<String, Cached>,
}

impl SponsorBlock {
    pub fn new(config: &SponsorBlockConfig) -> anyhow::Result<Self> {
        if !config.enabled {
            return Ok(Self { inner: None });
        }
        let http = reqwest::Client::builder()
            .tls_backend_preconfigured(crate::proxy::tls_config()?)
            .user_agent(USER_AGENT)
            .connect_timeout(CONNECT_TIMEOUT)
            .timeout(REQUEST_TIMEOUT)
            .build()?;
        let cache = Cache::builder()
            .max_capacity(CACHE_CAPACITY)
            .expire_after(CachedExpiry)
            .build();
        Ok(Self {
            inner: Some(Arc::new(Inner {
                http,
                api_url: config.api_url.clone(),
                cache,
            })),
        })
    }

    pub fn enabled(&self) -> bool {
        self.inner.is_some()
    }

    /// Segments of a valid video ID, sorted by start. Upstream failures yield no segments, so
    /// playback never depends on SponsorBlock.
    pub async fn segments(&self, video_id: &str) -> Arc<Vec<Segment>> {
        let Some(inner) = &self.inner else {
            return Arc::default();
        };
        inner
            .cache
            .get_with(video_id.to_owned(), async move {
                match inner.fetch(video_id).await {
                    Ok(segments) => Cached {
                        ttl: if segments.is_empty() { EMPTY_TTL } else { TTL },
                        segments: Arc::new(segments),
                    },
                    Err(err) => {
                        tracing::warn!(error = %err, "SponsorBlock request failed");
                        Cached {
                            segments: Arc::default(),
                            ttl: ERROR_TTL,
                        }
                    }
                }
            })
            .await
            .segments
    }
}

impl Inner {
    async fn fetch(&self, video_id: &str) -> anyhow::Result<Vec<Segment>> {
        let mut url = reqwest::Url::parse(&format!(
            "{}/api/skipSegments/{}",
            self.api_url,
            hash_prefix(video_id)
        ))?;
        let categories: Vec<_> = Category::ALL.iter().map(|c| c.as_str()).collect();
        url.query_pairs_mut()
            .append_pair("categories", &serde_json::to_string(&categories)?)
            .append_pair("actionTypes", ACTION_TYPES);
        let mut response = self.http.get(url).send().await?;
        // No segments for any video with this prefix.
        if response.status() == reqwest::StatusCode::NOT_FOUND {
            return Ok(Vec::new());
        }
        response = response.error_for_status()?;
        let mut body = Vec::new();
        while let Some(chunk) = response.chunk().await? {
            if body.len() + chunk.len() > MAX_RESPONSE_BYTES {
                anyhow::bail!("response too large");
            }
            body.extend_from_slice(&chunk);
        }
        let videos: Vec<UpstreamVideo> = serde_json::from_slice(&body)?;
        Ok(segments_of(videos, video_id))
    }
}

pub fn hash_prefix(video_id: &str) -> String {
    let hash = Sha256::digest(video_id.as_bytes());
    let mut hex = String::with_capacity(PREFIX_LEN);
    for byte in &hash[..PREFIX_LEN / 2] {
        let _ = write!(hex, "{byte:02x}");
    }
    hex
}

fn segments_of(videos: Vec<UpstreamVideo>, video_id: &str) -> Vec<Segment> {
    let mut segments: Vec<Segment> = videos
        .into_iter()
        .filter(|v| v.video_id == video_id)
        .flat_map(|v| v.segments)
        .filter_map(|s| {
            let category = Category::parse(&s.category)?;
            let action = Action::parse(&s.action_type)?;
            let [start, end] = s.segment[..] else {
                return None;
            };
            let valid = start.is_finite() && end.is_finite() && start >= 0.0;
            let valid = valid
                && match action {
                    Action::Skip => end > start,
                    Action::Poi | Action::Full => end >= start,
                };
            valid.then_some(Segment {
                id: s.uuid,
                category,
                action,
                start,
                end,
            })
        })
        .collect();
    let mut seen = std::collections::HashSet::new();
    segments.retain(|s| seen.insert(s.id.clone()));
    segments.sort_by(|a, b| a.start.total_cmp(&b.start).then(a.end.total_cmp(&b.end)));
    segments
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn prefix_is_first_four_hex_chars_of_sha256() {
        assert_eq!(hash_prefix("aqz-KE-bpKQ"), "671e");
    }

    #[test]
    fn keeps_valid_segments_of_the_requested_video() {
        let videos: Vec<UpstreamVideo> = serde_json::from_value(serde_json::json!([
            { "videoID": "other", "segments": [
                { "segment": [1.0, 2.0], "UUID": "x", "category": "sponsor", "actionType": "skip" }
            ]},
            { "videoID": "wanted", "segments": [
                { "segment": [50.0, 60.0], "UUID": "b", "category": "selfpromo", "actionType": "skip" },
                { "segment": [10.0, 20.0], "UUID": "a", "category": "sponsor", "actionType": "skip" },
                { "segment": [30.0, 30.0], "UUID": "c", "category": "poi_highlight", "actionType": "poi" },
                { "segment": [0.0, 0.0], "UUID": "d", "category": "exclusive_access", "actionType": "full" },
                { "segment": [5.0, 5.0], "UUID": "e", "category": "sponsor", "actionType": "skip" },
                { "segment": [5.0, 9.0], "UUID": "f", "category": "chapter", "actionType": "chapter" },
                { "segment": [5.0, 9.0], "UUID": "g", "category": "sponsor", "actionType": "mute" },
                { "segment": [5.0], "UUID": "h", "category": "sponsor", "actionType": "skip" }
            ]}
        ]))
        .unwrap();
        let segments = segments_of(videos, "wanted");
        let ids: Vec<_> = segments.iter().map(|s| s.id.as_str()).collect();
        assert_eq!(ids, ["d", "a", "c", "b"]);
        assert_eq!(segments[1].category, Category::Sponsor);
        assert_eq!(segments[2].action, Action::Poi);
    }
}
