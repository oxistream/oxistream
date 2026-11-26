//! YouTube access via RustyPipe, always unauthenticated.

use std::sync::Arc;
use std::time::{Duration, Instant};

use moka::Expiry;
use moka::future::Cache;
use rustypipe::client::{RustyPipe, RustyPipeQuery};
use rustypipe::model::paginator::{ContinuationEndpoint, Paginator};
use rustypipe::model::{
    Channel, SearchResult, UrlTarget, VideoDetails, VideoItem, VideoPlayer, YouTubeItem,
};
use rustypipe::param::Language;
use rustypipe::validate;
use time::OffsetDateTime;

use crate::config::YouTubeConfig;

mod error;

pub use error::YouTubeError;

pub type Result<T> = std::result::Result<T, YouTubeError>;

const PLAYER_CACHE_CAPACITY: u64 = 2_000;
/// Stream URLs must stay usable for a while after they are handed out.
const PLAYER_EXPIRY_MARGIN: Duration = Duration::from_mins(30);
const BOTGUARD_SNAPSHOT_FILE: &str = "oxistream-bg_snapshot.bin";

#[derive(Clone)]
pub struct YouTube {
    rp: RustyPipe,
    players: Cache<String, Arc<VideoPlayer>>,
}

impl YouTube {
    pub fn new(config: &YouTubeConfig) -> anyhow::Result<Self> {
        let mut builder = RustyPipe::builder()
            .unauthenticated()
            .no_reporter()
            .po_token_cache();
        builder = match &config.cache_dir {
            Some(dir) => builder.storage_dir(dir),
            None => builder
                .no_storage()
                .botguard_snapshot_file(std::env::temp_dir().join(BOTGUARD_SNAPSHOT_FILE)),
        };
        if let Some(bin) = &config.botguard_bin {
            builder = builder.botguard_bin(bin.as_os_str());
        }
        let rp = builder
            .build()
            .map_err(|err| anyhow::anyhow!("could not set up RustyPipe: {err}"))?;
        let players = Cache::builder()
            .max_capacity(PLAYER_CACHE_CAPACITY)
            .expire_after(PlayerExpiry)
            .build();
        Ok(Self { rp, players })
    }

    /// Version string of the botguard binary in use, if one was found.
    pub async fn botguard_version(&self) -> Option<String> {
        self.rp.version_botguard().await
    }

    fn query(&self, lang: Language) -> RustyPipeQuery {
        self.rp.query().lang(lang)
    }

    /// Player data with stream URLs, cached until shortly before the URLs expire.
    /// Concurrent misses for the same video share one upstream request.
    pub async fn player(&self, video_id: &str) -> Result<Arc<VideoPlayer>> {
        check_video_id(video_id)?;
        let query = self.rp.query();
        self.players
            .try_get_with(video_id.to_owned(), async move {
                query
                    .player(video_id)
                    .await
                    .map(Arc::new)
                    .map_err(YouTubeError::from)
            })
            .await
            .map_err(|err| (*err).clone())
    }

    /// Drops cached player data, e.g. after the stream URLs were rejected upstream.
    pub async fn invalidate(&self, video_id: &str) {
        self.players.invalidate(video_id).await;
    }

    pub async fn video_details(&self, video_id: &str, lang: Language) -> Result<VideoDetails> {
        check_video_id(video_id)?;
        Ok(self.query(lang).video_details(video_id).await?)
    }

    pub async fn search(&self, query: &str, lang: Language) -> Result<SearchResult<YouTubeItem>> {
        Ok(self.query(lang).search(query).await?)
    }

    pub async fn search_continuation(
        &self,
        token: &str,
        lang: Language,
    ) -> Result<Paginator<YouTubeItem>> {
        Ok(self
            .query(lang)
            .continuation(token, ContinuationEndpoint::Search, None)
            .await?)
    }

    pub async fn search_suggestions(&self, query: &str, lang: Language) -> Result<Vec<String>> {
        Ok(self.query(lang).search_suggestion(query).await?)
    }

    pub async fn channel_videos(
        &self,
        channel_id: &str,
        lang: Language,
    ) -> Result<Channel<Paginator<VideoItem>>> {
        validate::channel_id(channel_id)
            .map_err(|_| YouTubeError::InvalidInput("invalid channel id".into()))?;
        Ok(self.query(lang).channel_videos(channel_id).await?)
    }

    pub async fn channel_videos_continuation(
        &self,
        token: &str,
        lang: Language,
    ) -> Result<Paginator<VideoItem>> {
        Ok(self
            .query(lang)
            .continuation(token, ContinuationEndpoint::Browse, None)
            .await?)
    }

    /// Resolves a YouTube URL, URL path (`/@handle`), ID or handle.
    pub async fn resolve(&self, input: &str) -> Result<UrlTarget> {
        let input = normalize_resolve_input(input)?;
        self.rp
            .query()
            .resolve_string(input, false)
            .await
            .map_err(|err| match err {
                rustypipe::error::Error::Other(msg) => YouTubeError::InvalidInput(msg.into()),
                err => err.into(),
            })
    }
}

const YOUTUBE_HOSTS: &[&str] = &["youtube.com", "youtu.be", "youtube-nocookie.com"];

/// Makes paths absolute and rejects URLs of other sites, which RustyPipe would resolve by path.
fn normalize_resolve_input(input: &str) -> Result<String> {
    let input = input.trim();
    if input.starts_with('/') {
        return Ok(format!("https://www.youtube.com{input}"));
    }
    let url = match input.split_once("://") {
        Some((_, rest)) => rest,
        None if input.contains('/') && input.contains('.') => input,
        None => return Ok(input.to_owned()),
    };
    let host = url
        .split(['/', '?', '#'])
        .next()
        .unwrap_or_default()
        .to_ascii_lowercase();
    let host = host.split(':').next().unwrap_or_default();
    let is_youtube = YOUTUBE_HOSTS
        .iter()
        .any(|yt| host == *yt || host.strip_suffix(yt).is_some_and(|sub| sub.ends_with('.')));
    if !is_youtube {
        return Err(YouTubeError::InvalidInput("not a YouTube URL".into()));
    }
    Ok(format!("https://{url}"))
}

fn check_video_id(video_id: &str) -> Result<()> {
    validate::video_id(video_id).map_err(|_| YouTubeError::InvalidInput("invalid video id".into()))
}

struct PlayerExpiry;

impl Expiry<String, Arc<VideoPlayer>> for PlayerExpiry {
    fn expire_after_create(
        &self,
        _key: &String,
        player: &Arc<VideoPlayer>,
        _created_at: Instant,
    ) -> Option<Duration> {
        Some(player_ttl(player.valid_until, OffsetDateTime::now_utc()))
    }
}

fn player_ttl(valid_until: OffsetDateTime, now: OffsetDateTime) -> Duration {
    if valid_until <= now {
        return Duration::ZERO;
    }
    (valid_until - now)
        .unsigned_abs()
        .saturating_sub(PLAYER_EXPIRY_MARGIN)
}

#[cfg(test)]
mod tests {
    use time::macros::datetime;

    use super::*;

    #[test]
    fn player_ttl_keeps_a_safety_margin() {
        let now = datetime!(2026-01-01 12:00 UTC);
        assert_eq!(
            player_ttl(datetime!(2026-01-01 18:00 UTC), now),
            Duration::from_mins(5 * 60 + 30)
        );
        assert_eq!(
            player_ttl(datetime!(2026-01-01 12:10 UTC), now),
            Duration::ZERO
        );
        assert_eq!(
            player_ttl(datetime!(2026-01-01 11:00 UTC), now),
            Duration::ZERO
        );
    }

    #[test]
    fn normalizes_resolve_input() {
        let ok = |input: &str| normalize_resolve_input(input).unwrap();
        assert_eq!(ok("/@handle"), "https://www.youtube.com/@handle");
        assert_eq!(ok(" youtu.be/abc "), "https://youtu.be/abc");
        assert_eq!(
            ok("http://m.youtube.com/watch?v=x"),
            "https://m.youtube.com/watch?v=x"
        );
        assert_eq!(ok("dQw4w9WgXcQ"), "dQw4w9WgXcQ");
        assert_eq!(ok("@handle"), "@handle");
        for bad in [
            "https://example.com/watch?v=x",
            "https://notyoutube.com/@x",
            "evil.com/youtube.com/x",
        ] {
            assert!(normalize_resolve_input(bad).is_err(), "{bad}");
        }
    }

    #[tokio::test]
    async fn rejects_invalid_ids_without_network() {
        let youtube = YouTube::new(&YouTubeConfig::default()).unwrap();
        assert!(matches!(
            youtube.player("nope").await,
            Err(YouTubeError::InvalidInput(_))
        ));
        assert!(matches!(
            youtube.channel_videos("nope", Language::En).await,
            Err(YouTubeError::InvalidInput(_))
        ));
    }
}
