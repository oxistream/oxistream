//! Feed worker: keeps the videos of subscribed channels and followed playlists up to date.

use std::sync::Arc;
use std::time::Duration;

use async_trait::async_trait;
use oxistream_db::{
    AdvisoryLock, ChannelRepository, ChannelUpdate, ChannelVideoUpsert, FeedStatus, Image, LockKey,
    PgPool, YouTubePlaylistItemUpsert, YouTubePlaylistRepository, YouTubePlaylistUpdate,
};
use rustypipe::model::paginator::Paginator;
use rustypipe::model::{self as rp};
use rustypipe::param::{ChannelOrder, ChannelVideoTab, Language};
use time::OffsetDateTime;
use tokio::sync::mpsc;

use crate::youtube::{self, YouTube, YouTubeError};

/// Upper bound for the scheduler tick, so channels become due spread over the interval.
const MAX_TICK: Duration = Duration::from_mins(1);
const DUE_BATCH: u32 = 100;
/// Items stored per followed playlist: up to five page requests per fetch. Changes further
/// down a longer playlist go unnoticed; its metadata stays current.
pub const MAX_STORED_PLAYLIST_ITEMS: usize = 500;

/// A channel's metadata and latest videos, as fetched from YouTube.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ChannelFetch {
    pub channel: ChannelUpdate,
    pub videos: Vec<ChannelVideoUpsert>,
}

/// Where the worker gets channel videos from; YouTube in production, fakes in tests.
#[async_trait]
pub trait ChannelSource: Send + Sync {
    async fn fetch(&self, channel_id: &str) -> youtube::Result<ChannelFetch>;
}

#[async_trait]
impl ChannelSource for YouTube {
    async fn fetch(&self, channel_id: &str) -> youtube::Result<ChannelFetch> {
        let channel = self.channel_videos(channel_id, Language::En).await?;
        // The videos tab leaves out shorts; without them the feed simply has none.
        let shorts = if channel.has_shorts {
            match self
                .channel_tab(
                    channel_id,
                    ChannelVideoTab::Shorts,
                    ChannelOrder::Latest,
                    Language::En,
                )
                .await
            {
                Ok(page) => page.items,
                Err(err) => {
                    tracing::warn!(%channel_id, error = ?err, "channel shorts fetch failed");
                    Vec::new()
                }
            }
        } else {
            Vec::new()
        };
        Ok(ChannelFetch::from_rp(channel, shorts))
    }
}

fn images(src: &[rp::Thumbnail]) -> Vec<Image> {
    src.iter()
        .map(|t| Image {
            url: t.url.clone(),
            width: t.width,
            height: t.height,
        })
        .collect()
}

impl ChannelFetch {
    pub fn from_rp(
        channel: rp::Channel<Paginator<rp::VideoItem>>,
        shorts: Vec<rp::VideoItem>,
    ) -> Self {
        let videos = channel
            .content
            .items
            .into_iter()
            .chain(shorts)
            .map(|v| ChannelVideoUpsert {
                thumbnails: images(&v.thumbnail),
                duration_seconds: v.duration.and_then(|d| i32::try_from(d).ok()),
                published_at: v.publish_date,
                published_text: v.publish_date_txt,
                view_count: v.view_count.and_then(|c| i64::try_from(c).ok()),
                is_live: v.is_live,
                is_upcoming: v.is_upcoming,
                is_short: v.is_short,
                video_id: v.id,
                title: v.name,
            })
            .collect();
        Self {
            channel: ChannelUpdate {
                avatar: images(&channel.avatar),
                subscriber_count: channel.subscriber_count.and_then(|c| i64::try_from(c).ok()),
                verified: channel.verification.verified(),
                id: channel.id,
                name: channel.name,
                handle: channel.handle,
            },
            videos,
        }
    }
}

/// A playlist's metadata and first items, as fetched from YouTube.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PlaylistFetch {
    pub playlist: YouTubePlaylistUpdate,
    pub items: Vec<YouTubePlaylistItemUpsert>,
}

/// Where the worker gets followed playlists from; YouTube in production, fakes in tests.
#[async_trait]
pub trait PlaylistSource: Send + Sync {
    async fn fetch_playlist(&self, playlist_id: &str) -> youtube::Result<PlaylistFetch>;
}

#[async_trait]
impl PlaylistSource for YouTube {
    async fn fetch_playlist(&self, playlist_id: &str) -> youtube::Result<PlaylistFetch> {
        let (playlist, videos) = self
            .playlist_with_videos(playlist_id, MAX_STORED_PLAYLIST_ITEMS, Language::En)
            .await?;
        Ok(PlaylistFetch::from_rp(playlist, videos))
    }
}

impl PlaylistFetch {
    pub fn from_rp(playlist: rp::Playlist, videos: Vec<rp::VideoItem>) -> Self {
        let items = videos
            .into_iter()
            .map(|v| YouTubePlaylistItemUpsert {
                duration_seconds: v.duration.and_then(|d| i32::try_from(d).ok()),
                channel_id: v.channel.as_ref().map(|c| c.id.clone()),
                channel_name: v.channel.map(|c| c.name),
                video_id: v.id,
                title: v.name,
            })
            .collect();
        Self {
            playlist: YouTubePlaylistUpdate {
                thumbnails: images(&playlist.thumbnail),
                video_count: i64::try_from(playlist.video_count).ok(),
                channel_id: playlist.channel.as_ref().map(|c| c.id.clone()),
                channel_name: playlist.channel.map(|c| c.name),
                id: playlist.id,
                title: playlist.name,
            },
            items,
        }
    }
}

/// Handle for request handlers: asks the worker for immediate fetches and reads its status.
#[derive(Clone)]
pub struct Feed {
    requests: mpsc::UnboundedSender<String>,
    playlist_requests: mpsc::UnboundedSender<String>,
    channels: Arc<dyn ChannelRepository>,
}

/// Fetch requests sent through a [`Feed`], consumed by [`FeedWorker::spawn`].
pub struct FeedRequests {
    channels: mpsc::UnboundedReceiver<String>,
    playlists: mpsc::UnboundedReceiver<String>,
}

impl FeedRequests {
    /// The next requested channel.
    pub async fn recv(&mut self) -> Option<String> {
        self.channels.recv().await
    }

    pub fn try_recv(&mut self) -> Option<String> {
        self.channels.try_recv().ok()
    }

    /// The next requested playlist.
    pub async fn recv_playlist(&mut self) -> Option<String> {
        self.playlists.recv().await
    }

    pub fn try_recv_playlist(&mut self) -> Option<String> {
        self.playlists.try_recv().ok()
    }
}

impl Feed {
    pub fn new(channels: Arc<dyn ChannelRepository>) -> (Self, FeedRequests) {
        let (requests, channel_rx) = mpsc::unbounded_channel();
        let (playlist_requests, playlist_rx) = mpsc::unbounded_channel();
        (
            Self {
                requests,
                playlist_requests,
                channels,
            },
            FeedRequests {
                channels: channel_rx,
                playlists: playlist_rx,
            },
        )
    }

    /// Queues a fetch of `channel_id` without waiting for it.
    pub fn request_fetch(&self, channel_id: &str) {
        if self.requests.send(channel_id.to_owned()).is_err() {
            tracing::debug!(%channel_id, "feed worker is not running");
        }
    }

    /// Queues a fetch of a followed playlist without waiting for it.
    pub fn request_playlist_fetch(&self, playlist_id: &str) {
        if self.playlist_requests.send(playlist_id.to_owned()).is_err() {
            tracing::debug!(%playlist_id, "feed worker is not running");
        }
    }

    pub async fn status(&self) -> Result<FeedStatus, oxistream_db::Error> {
        self.channels.status().await
    }
}

/// Result of a scheduled run that held the lock.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub struct RunSummary {
    pub fetched: usize,
    pub failed: usize,
    /// Videos of channels without subscribers that were removed.
    pub pruned: u64,
    pub playlists_fetched: usize,
    pub playlists_failed: usize,
    /// Items of playlists without followers that were removed.
    pub playlists_pruned: u64,
    /// The run stopped early because YouTube is blocking requests.
    pub rate_limited: bool,
}

#[derive(Clone)]
pub struct FeedWorker {
    pool: PgPool,
    channels: Arc<dyn ChannelRepository>,
    source: Arc<dyn ChannelSource>,
    playlists: Option<Playlists>,
    interval: Duration,
}

#[derive(Clone)]
struct Playlists {
    repo: Arc<dyn YouTubePlaylistRepository>,
    source: Arc<dyn PlaylistSource>,
}

impl FeedWorker {
    pub fn new(
        pool: PgPool,
        channels: Arc<dyn ChannelRepository>,
        source: Arc<dyn ChannelSource>,
        interval: Duration,
    ) -> Self {
        Self {
            pool,
            channels,
            source,
            playlists: None,
            interval,
        }
    }

    /// Also keeps followed playlists up to date.
    #[must_use]
    pub fn with_playlists(
        mut self,
        repo: Arc<dyn YouTubePlaylistRepository>,
        source: Arc<dyn PlaylistSource>,
    ) -> Self {
        self.playlists = Some(Playlists { repo, source });
        self
    }

    /// Runs the scheduler and serves fetch requests in the background.
    pub fn spawn(self, requests: FeedRequests) {
        let FeedRequests {
            mut channels,
            mut playlists,
        } = requests;
        let worker = self.clone();
        tokio::spawn(async move {
            while let Some(channel_id) = channels.recv().await {
                if let Err(err) = worker.fetch_requested(&channel_id).await {
                    tracing::warn!(%channel_id, error = ?err, "requested channel fetch failed");
                }
            }
        });
        let worker = self.clone();
        tokio::spawn(async move {
            while let Some(playlist_id) = playlists.recv().await {
                if let Err(err) = worker.fetch_requested_playlist(&playlist_id).await {
                    tracing::warn!(%playlist_id, error = ?err, "requested playlist fetch failed");
                }
            }
        });
        tokio::spawn(async move {
            let mut tick = tokio::time::interval(self.interval.min(MAX_TICK));
            tick.set_missed_tick_behavior(tokio::time::MissedTickBehavior::Delay);
            loop {
                tick.tick().await;
                match self.run_once().await {
                    Ok(Some(summary)) => {
                        let attempts = summary.fetched
                            + summary.failed
                            + summary.playlists_fetched
                            + summary.playlists_failed;
                        if attempts > 0 {
                            tracing::info!(?summary, "feed worker run finished");
                        }
                        if summary.rate_limited {
                            tokio::time::sleep(self.interval).await;
                        }
                    }
                    Ok(None) => {}
                    Err(err) => tracing::warn!(error = ?err, "feed worker run failed"),
                }
            }
        });
    }

    /// Fetches every subscribed channel and followed playlist not checked within the interval,
    /// unless another replica holds the worker lock; then returns `None`.
    pub async fn run_once(&self) -> Result<Option<RunSummary>, oxistream_db::Error> {
        let Some(lock) = AdvisoryLock::try_acquire(&self.pool, LockKey::FeedWorker).await? else {
            return Ok(None);
        };
        let result = self.run_locked().await;
        lock.release().await?;
        result.map(Some)
    }

    async fn run_locked(&self) -> Result<RunSummary, oxistream_db::Error> {
        let mut summary = RunSummary {
            pruned: self.channels.prune_unsubscribed().await?,
            ..RunSummary::default()
        };
        let cutoff = OffsetDateTime::now_utc() - self.interval;
        loop {
            let due = self.channels.due(cutoff, DUE_BATCH).await?;
            if due.is_empty() {
                break;
            }
            for channel_id in due {
                match self.fetch_channel(&channel_id).await? {
                    Fetched::Ok => summary.fetched += 1,
                    Fetched::Failed => summary.failed += 1,
                    Fetched::RateLimited => {
                        summary.rate_limited = true;
                        return Ok(summary);
                    }
                }
            }
        }
        let Some(playlists) = &self.playlists else {
            return Ok(summary);
        };
        summary.playlists_pruned = playlists.repo.prune_unfollowed().await?;
        loop {
            let due = playlists.repo.due(cutoff, DUE_BATCH).await?;
            if due.is_empty() {
                return Ok(summary);
            }
            for playlist_id in due {
                match self.fetch_playlist(playlists, &playlist_id).await? {
                    Fetched::Ok => summary.playlists_fetched += 1,
                    Fetched::Failed => summary.playlists_failed += 1,
                    Fetched::RateLimited => {
                        summary.rate_limited = true;
                        return Ok(summary);
                    }
                }
            }
        }
    }

    /// Fetches a channel right away unless it was fetched within the interval.
    /// Returns whether it was fetched.
    pub async fn fetch_requested(&self, channel_id: &str) -> Result<bool, oxistream_db::Error> {
        let fresh_after = OffsetDateTime::now_utc() - self.interval;
        let fresh = self
            .channels
            .get(channel_id)
            .await?
            .and_then(|c| c.fetched_at)
            .is_some_and(|at| at > fresh_after);
        if fresh {
            return Ok(false);
        }
        Ok(matches!(self.fetch_channel(channel_id).await?, Fetched::Ok))
    }

    /// Fetches a followed playlist right away unless it was fetched within the interval.
    /// Returns whether it was fetched.
    pub async fn fetch_requested_playlist(
        &self,
        playlist_id: &str,
    ) -> Result<bool, oxistream_db::Error> {
        let Some(playlists) = &self.playlists else {
            return Ok(false);
        };
        let fresh_after = OffsetDateTime::now_utc() - self.interval;
        let fresh = playlists
            .repo
            .get(playlist_id)
            .await?
            .and_then(|p| p.fetched_at)
            .is_some_and(|at| at > fresh_after);
        if fresh {
            return Ok(false);
        }
        Ok(matches!(
            self.fetch_playlist(playlists, playlist_id).await?,
            Fetched::Ok
        ))
    }

    async fn fetch_channel(&self, channel_id: &str) -> Result<Fetched, oxistream_db::Error> {
        match self.source.fetch(channel_id).await {
            Ok(fetch) => {
                // The ID YouTube returns is canonical, but rows are keyed by the requested one.
                let channel = ChannelUpdate {
                    id: channel_id.to_owned(),
                    ..fetch.channel
                };
                self.channels.store_fetch(&channel, &fetch.videos).await?;
                metrics::counter!("feed_channel_fetches_total", "result" => "ok").increment(1);
                Ok(Fetched::Ok)
            }
            Err(YouTubeError::RateLimited(detail)) => {
                tracing::warn!(%channel_id, %detail, "YouTube is blocking channel fetches");
                metrics::counter!("feed_channel_fetches_total", "result" => "rate_limited")
                    .increment(1);
                Ok(Fetched::RateLimited)
            }
            Err(err) => {
                tracing::debug!(%channel_id, error = %err, "channel fetch failed");
                self.channels
                    .record_failure(channel_id, &err.to_string())
                    .await?;
                metrics::counter!("feed_channel_fetches_total", "result" => "error").increment(1);
                Ok(Fetched::Failed)
            }
        }
    }

    async fn fetch_playlist(
        &self,
        playlists: &Playlists,
        playlist_id: &str,
    ) -> Result<Fetched, oxistream_db::Error> {
        match playlists.source.fetch_playlist(playlist_id).await {
            Ok(fetch) => {
                let playlist = YouTubePlaylistUpdate {
                    id: playlist_id.to_owned(),
                    ..fetch.playlist
                };
                playlists.repo.store_fetch(&playlist, &fetch.items).await?;
                metrics::counter!("feed_playlist_fetches_total", "result" => "ok").increment(1);
                Ok(Fetched::Ok)
            }
            Err(YouTubeError::RateLimited(detail)) => {
                tracing::warn!(%playlist_id, %detail, "YouTube is blocking playlist fetches");
                metrics::counter!("feed_playlist_fetches_total", "result" => "rate_limited")
                    .increment(1);
                Ok(Fetched::RateLimited)
            }
            Err(err) => {
                tracing::debug!(%playlist_id, error = %err, "playlist fetch failed");
                playlists
                    .repo
                    .record_failure(playlist_id, &err.to_string())
                    .await?;
                metrics::counter!("feed_playlist_fetches_total", "result" => "error").increment(1);
                Ok(Fetched::Failed)
            }
        }
    }
}

enum Fetched {
    Ok,
    Failed,
    RateLimited,
}
