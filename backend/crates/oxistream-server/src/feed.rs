//! Feed worker: keeps the videos of subscribed channels up to date.

use std::sync::Arc;
use std::time::Duration;

use async_trait::async_trait;
use oxistream_db::{
    AdvisoryLock, ChannelRepository, ChannelUpdate, ChannelVideoUpsert, FeedStatus, Image, LockKey,
    PgPool,
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

/// Handle for request handlers: asks the worker for immediate fetches and reads its status.
#[derive(Clone)]
pub struct Feed {
    requests: mpsc::UnboundedSender<String>,
    channels: Arc<dyn ChannelRepository>,
}

/// Fetch requests sent through a [`Feed`], consumed by [`FeedWorker::spawn`].
pub struct FeedRequests(mpsc::UnboundedReceiver<String>);

impl FeedRequests {
    pub async fn recv(&mut self) -> Option<String> {
        self.0.recv().await
    }

    pub fn try_recv(&mut self) -> Option<String> {
        self.0.try_recv().ok()
    }
}

impl Feed {
    pub fn new(channels: Arc<dyn ChannelRepository>) -> (Self, FeedRequests) {
        let (requests, rx) = mpsc::unbounded_channel();
        (Self { requests, channels }, FeedRequests(rx))
    }

    /// Queues a fetch of `channel_id` without waiting for it.
    pub fn request_fetch(&self, channel_id: &str) {
        if self.requests.send(channel_id.to_owned()).is_err() {
            tracing::debug!(%channel_id, "feed worker is not running");
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
    /// The run stopped early because YouTube is blocking requests.
    pub rate_limited: bool,
}

#[derive(Clone)]
pub struct FeedWorker {
    pool: PgPool,
    channels: Arc<dyn ChannelRepository>,
    source: Arc<dyn ChannelSource>,
    interval: Duration,
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
            interval,
        }
    }

    /// Runs the scheduler and serves fetch requests in the background.
    pub fn spawn(self, mut requests: FeedRequests) {
        let worker = self.clone();
        tokio::spawn(async move {
            while let Some(channel_id) = requests.recv().await {
                if let Err(err) = worker.fetch_requested(&channel_id).await {
                    tracing::warn!(%channel_id, error = ?err, "requested channel fetch failed");
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
                        if summary.fetched + summary.failed > 0 {
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

    /// Fetches every subscribed channel not checked within the interval, unless another
    /// replica holds the worker lock; then returns `None`.
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
                return Ok(summary);
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
}

enum Fetched {
    Ok,
    Failed,
    RateLimited,
}
