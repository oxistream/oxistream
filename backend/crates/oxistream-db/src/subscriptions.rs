use serde::{Deserialize, Serialize};
use sqlx::PgPool;
use sqlx::types::Json;
use time::OffsetDateTime;
use uuid::Uuid;

use crate::Error;

/// An upstream image URL with its size.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Image {
    pub url: String,
    pub width: u32,
    pub height: u32,
}

/// A YouTube channel someone subscribed to, with what the feed worker last saw of it.
#[derive(Debug, Clone, PartialEq, Eq, sqlx::FromRow)]
pub struct Channel {
    pub id: String,
    /// `None` until the channel was fetched once.
    pub name: Option<String>,
    pub handle: Option<String>,
    #[sqlx(json)]
    pub avatar: Vec<Image>,
    pub subscriber_count: Option<i64>,
    pub verified: bool,
    /// Last successful fetch.
    pub fetched_at: Option<OffsetDateTime>,
    /// Last fetch attempt.
    pub checked_at: Option<OffsetDateTime>,
    pub last_error: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Eq, sqlx::FromRow)]
pub struct Subscription {
    #[sqlx(flatten)]
    pub channel: Channel,
    pub subscribed_at: OffsetDateTime,
}

/// A video in a user's feed.
#[derive(Debug, Clone, PartialEq, Eq, sqlx::FromRow)]
#[allow(clippy::struct_excessive_bools)]
pub struct FeedVideo {
    pub video_id: String,
    pub channel_id: String,
    pub channel_name: Option<String>,
    #[sqlx(json)]
    pub channel_avatar: Vec<Image>,
    pub channel_verified: bool,
    pub title: String,
    pub duration_seconds: Option<i32>,
    pub published_at: Option<OffsetDateTime>,
    pub published_text: Option<String>,
    #[sqlx(json)]
    pub thumbnails: Vec<Image>,
    pub view_count: Option<i64>,
    pub is_live: bool,
    pub is_upcoming: bool,
    pub is_short: bool,
    /// Feed order: the publish time, else when the video was first seen.
    pub sort_at: OffsetDateTime,
}

impl FeedVideo {
    /// The cursor that continues the feed after this video.
    pub fn cursor(&self) -> FeedCursor {
        FeedCursor {
            sort_at: self.sort_at,
            video_id: self.video_id.clone(),
        }
    }
}

/// Position in a feed; the next page starts after it.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct FeedCursor {
    pub sort_at: OffsetDateTime,
    pub video_id: String,
}

/// A channel from an imported subscription list.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ImportedChannel {
    pub id: String,
    /// Name from the file; stored only while the channel has none.
    pub name: Option<String>,
}

#[async_trait::async_trait]
pub trait SubscriptionRepository: Send + Sync {
    /// Subscribes `user_id` to `channel_id`, creating the channel if unknown, and lifts a
    /// block of it. Returns the subscription and whether it is new.
    async fn subscribe(
        &self,
        user_id: Uuid,
        channel_id: &str,
    ) -> Result<(Subscription, bool), Error>;

    /// Returns whether a subscription existed.
    /// Subscribes `user_id` to all `channels` in one transaction, lifting blocks of them. IDs
    /// must be unique.
    /// Returns how many subscriptions are new.
    async fn import(&self, user_id: Uuid, channels: &[ImportedChannel]) -> Result<u64, Error>;

    async fn unsubscribe(&self, user_id: Uuid, channel_id: &str) -> Result<bool, Error>;

    async fn is_subscribed(&self, user_id: Uuid, channel_id: &str) -> Result<bool, Error>;

    /// All subscriptions of a user, ordered by channel name.
    async fn list(&self, user_id: Uuid) -> Result<Vec<Subscription>, Error>;

    /// Videos of subscribed channels, newest first, starting after `after`.
    async fn feed(
        &self,
        user_id: Uuid,
        filter: FeedFilter,
        after: Option<&FeedCursor>,
        limit: u32,
    ) -> Result<Vec<FeedVideo>, Error>;
}

/// Which videos of subscribed channels the feed lists.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub enum FeedFilter {
    #[default]
    All,
    Videos,
    Shorts,
}

impl FeedFilter {
    fn is_short(self) -> Option<bool> {
        match self {
            Self::All => None,
            Self::Videos => Some(false),
            Self::Shorts => Some(true),
        }
    }
}

/// Channel metadata written by a successful fetch.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ChannelUpdate {
    pub id: String,
    pub name: String,
    pub handle: Option<String>,
    pub avatar: Vec<Image>,
    pub subscriber_count: Option<i64>,
    pub verified: bool,
}

#[derive(Debug, Clone, PartialEq, Eq)]
#[allow(clippy::struct_excessive_bools)]
pub struct ChannelVideoUpsert {
    pub video_id: String,
    pub title: String,
    pub duration_seconds: Option<i32>,
    /// Kept once known, since later fetches only see a coarser relative date.
    pub published_at: Option<OffsetDateTime>,
    pub published_text: Option<String>,
    pub thumbnails: Vec<Image>,
    pub view_count: Option<i64>,
    pub is_live: bool,
    pub is_upcoming: bool,
    pub is_short: bool,
}

/// Feed worker state, derived from the database so every replica sees the same.
#[derive(Debug, Clone, PartialEq, Eq, sqlx::FromRow)]
pub struct FeedStatus {
    /// Channels with at least one subscriber.
    pub channels: i64,
    /// Subscribed channels that were never fetched successfully.
    pub pending: i64,
    /// Subscribed channels whose last fetch failed.
    pub failing: i64,
    /// Most recent fetch attempt of any subscribed channel.
    pub last_checked_at: Option<OffsetDateTime>,
}

/// Feed worker side of channels.
#[async_trait::async_trait]
pub trait ChannelRepository: Send + Sync {
    async fn get(&self, id: &str) -> Result<Option<Channel>, Error>;

    /// IDs of subscribed channels not checked since `checked_before`, least recently
    /// checked first.
    async fn due(&self, checked_before: OffsetDateTime, limit: u32) -> Result<Vec<String>, Error>;

    /// Stores a successful fetch: channel metadata and its latest videos.
    async fn store_fetch(
        &self,
        channel: &ChannelUpdate,
        videos: &[ChannelVideoUpsert],
    ) -> Result<(), Error>;

    async fn record_failure(&self, id: &str, error: &str) -> Result<(), Error>;

    /// Deletes the videos of channels nobody subscribes to anymore.
    async fn prune_unsubscribed(&self) -> Result<u64, Error>;

    async fn status(&self) -> Result<FeedStatus, Error>;
}

#[derive(Debug, Clone)]
pub struct PgSubscriptionRepository {
    pool: PgPool,
}

impl PgSubscriptionRepository {
    pub fn new(pool: PgPool) -> Self {
        Self { pool }
    }
}

const SUBSCRIPTION_COLUMNS: &str = "c.id, c.name, c.handle, c.avatar, c.subscriber_count, \
    c.verified, c.fetched_at, c.checked_at, c.last_error, s.created_at AS subscribed_at";

#[async_trait::async_trait]
impl SubscriptionRepository for PgSubscriptionRepository {
    async fn subscribe(
        &self,
        user_id: Uuid,
        channel_id: &str,
    ) -> Result<(Subscription, bool), Error> {
        let mut tx = self.pool.begin().await?;
        sqlx::query("INSERT INTO channels (id) VALUES ($1) ON CONFLICT (id) DO NOTHING")
            .bind(channel_id)
            .execute(&mut *tx)
            .await?;
        let created = sqlx::query(
            "INSERT INTO subscriptions (user_id, channel_id) VALUES ($1, $2) \
             ON CONFLICT DO NOTHING",
        )
        .bind(user_id)
        .bind(channel_id)
        .execute(&mut *tx)
        .await?
        .rows_affected()
            > 0;
        sqlx::query("DELETE FROM channel_blocks WHERE user_id = $1 AND channel_id = $2")
            .bind(user_id)
            .bind(channel_id)
            .execute(&mut *tx)
            .await?;
        let subscription = sqlx::query_as::<_, Subscription>(sqlx::AssertSqlSafe(format!(
            "SELECT {SUBSCRIPTION_COLUMNS} FROM subscriptions s \
             JOIN channels c ON c.id = s.channel_id \
             WHERE s.user_id = $1 AND s.channel_id = $2"
        )))
        .bind(user_id)
        .bind(channel_id)
        .fetch_one(&mut *tx)
        .await?;
        tx.commit().await?;
        Ok((subscription, created))
    }

    async fn import(&self, user_id: Uuid, channels: &[ImportedChannel]) -> Result<u64, Error> {
        let ids: Vec<&str> = channels.iter().map(|c| c.id.as_str()).collect();
        let names: Vec<Option<&str>> = channels.iter().map(|c| c.name.as_deref()).collect();
        let mut tx = self.pool.begin().await?;
        sqlx::query(
            "INSERT INTO channels (id, name) SELECT * FROM UNNEST($1::text[], $2::text[]) \
             ON CONFLICT (id) DO UPDATE SET name = COALESCE(channels.name, EXCLUDED.name) \
             WHERE channels.name IS NULL AND EXCLUDED.name IS NOT NULL",
        )
        .bind(&ids)
        .bind(&names)
        .execute(&mut *tx)
        .await?;
        let created = sqlx::query(
            "INSERT INTO subscriptions (user_id, channel_id) \
             SELECT $1, id FROM UNNEST($2::text[]) AS t (id) \
             ON CONFLICT DO NOTHING",
        )
        .bind(user_id)
        .bind(&ids)
        .execute(&mut *tx)
        .await?
        .rows_affected();
        sqlx::query("DELETE FROM channel_blocks WHERE user_id = $1 AND channel_id = ANY ($2)")
            .bind(user_id)
            .bind(&ids)
            .execute(&mut *tx)
            .await?;
        tx.commit().await?;
        Ok(created)
    }

    async fn unsubscribe(&self, user_id: Uuid, channel_id: &str) -> Result<bool, Error> {
        Ok(
            sqlx::query("DELETE FROM subscriptions WHERE user_id = $1 AND channel_id = $2")
                .bind(user_id)
                .bind(channel_id)
                .execute(&self.pool)
                .await?
                .rows_affected()
                > 0,
        )
    }

    async fn is_subscribed(&self, user_id: Uuid, channel_id: &str) -> Result<bool, Error> {
        Ok(sqlx::query_scalar(
            "SELECT EXISTS (SELECT 1 FROM subscriptions WHERE user_id = $1 AND channel_id = $2)",
        )
        .bind(user_id)
        .bind(channel_id)
        .fetch_one(&self.pool)
        .await?)
    }

    async fn list(&self, user_id: Uuid) -> Result<Vec<Subscription>, Error> {
        Ok(
            sqlx::query_as::<_, Subscription>(sqlx::AssertSqlSafe(format!(
                "SELECT {SUBSCRIPTION_COLUMNS} FROM subscriptions s \
             JOIN channels c ON c.id = s.channel_id \
             WHERE s.user_id = $1 \
             ORDER BY lower(c.name) NULLS LAST, c.id"
            )))
            .bind(user_id)
            .fetch_all(&self.pool)
            .await?,
        )
    }

    async fn feed(
        &self,
        user_id: Uuid,
        filter: FeedFilter,
        after: Option<&FeedCursor>,
        limit: u32,
    ) -> Result<Vec<FeedVideo>, Error> {
        Ok(sqlx::query_as::<_, FeedVideo>(
            "SELECT v.video_id, v.channel_id, c.name AS channel_name, \
                c.avatar AS channel_avatar, c.verified AS channel_verified, v.title, \
                v.duration_seconds, v.published_at, v.published_text, v.thumbnails, \
                v.view_count, v.is_live, v.is_upcoming, v.is_short, v.sort_at \
             FROM channel_videos v \
             JOIN subscriptions s ON s.channel_id = v.channel_id AND s.user_id = $1 \
             JOIN channels c ON c.id = v.channel_id \
             WHERE ($2::timestamptz IS NULL OR (v.sort_at, v.video_id) < ($2, $3)) \
                AND ($5::boolean IS NULL OR v.is_short = $5) \
             ORDER BY v.sort_at DESC, v.video_id DESC \
             LIMIT $4",
        )
        .bind(user_id)
        .bind(after.map(|c| c.sort_at))
        .bind(after.map(|c| c.video_id.as_str()))
        .bind(i64::from(limit))
        .bind(filter.is_short())
        .fetch_all(&self.pool)
        .await?)
    }
}

#[derive(Debug, Clone)]
pub struct PgChannelRepository {
    pool: PgPool,
}

impl PgChannelRepository {
    pub fn new(pool: PgPool) -> Self {
        Self { pool }
    }
}

#[async_trait::async_trait]
impl ChannelRepository for PgChannelRepository {
    async fn get(&self, id: &str) -> Result<Option<Channel>, Error> {
        Ok(sqlx::query_as::<_, Channel>(
            "SELECT id, name, handle, avatar, subscriber_count, verified, fetched_at, \
                checked_at, last_error \
             FROM channels WHERE id = $1",
        )
        .bind(id)
        .fetch_optional(&self.pool)
        .await?)
    }

    async fn due(&self, checked_before: OffsetDateTime, limit: u32) -> Result<Vec<String>, Error> {
        Ok(sqlx::query_scalar(
            "SELECT c.id FROM channels c \
             WHERE EXISTS (SELECT 1 FROM subscriptions s WHERE s.channel_id = c.id) \
                AND (c.checked_at IS NULL OR c.checked_at < $1) \
             ORDER BY c.checked_at NULLS FIRST, c.id \
             LIMIT $2",
        )
        .bind(checked_before)
        .bind(i64::from(limit))
        .fetch_all(&self.pool)
        .await?)
    }

    async fn store_fetch(
        &self,
        channel: &ChannelUpdate,
        videos: &[ChannelVideoUpsert],
    ) -> Result<(), Error> {
        let mut tx = self.pool.begin().await?;
        sqlx::query(
            "INSERT INTO channels \
                (id, name, handle, avatar, subscriber_count, verified, fetched_at, checked_at) \
             VALUES ($1, $2, $3, $4, $5, $6, now(), now()) \
             ON CONFLICT (id) DO UPDATE SET \
                name = EXCLUDED.name, \
                handle = COALESCE(EXCLUDED.handle, channels.handle), \
                avatar = EXCLUDED.avatar, \
                subscriber_count = COALESCE(EXCLUDED.subscriber_count, channels.subscriber_count), \
                verified = EXCLUDED.verified, \
                fetched_at = now(), \
                checked_at = now(), \
                last_error = NULL",
        )
        .bind(&channel.id)
        .bind(&channel.name)
        .bind(&channel.handle)
        .bind(Json(&channel.avatar))
        .bind(channel.subscriber_count)
        .bind(channel.verified)
        .execute(&mut *tx)
        .await?;
        for (position, video) in (0_i32..).zip(videos) {
            // A video stays with the channel it was first seen on. Videos without a date, such as
            // shorts, sort by first sight, staggered by list position to keep the channel's order.
            sqlx::query(
                "INSERT INTO channel_videos \
                    (video_id, channel_id, title, duration_seconds, published_at, \
                     published_text, thumbnails, view_count, is_live, is_upcoming, is_short, \
                     first_seen_at) \
                 VALUES ($1, $2, $3, $4, $5, $6, $7, $8, $9, $10, $11, \
                     now() - make_interval(secs => $12::double precision / 1000)) \
                 ON CONFLICT (video_id) DO UPDATE SET \
                    title = EXCLUDED.title, \
                    duration_seconds = COALESCE(EXCLUDED.duration_seconds, \
                        channel_videos.duration_seconds), \
                    published_at = COALESCE(channel_videos.published_at, EXCLUDED.published_at), \
                    published_text = EXCLUDED.published_text, \
                    thumbnails = EXCLUDED.thumbnails, \
                    view_count = COALESCE(EXCLUDED.view_count, channel_videos.view_count), \
                    is_live = EXCLUDED.is_live, \
                    is_upcoming = EXCLUDED.is_upcoming, \
                    is_short = EXCLUDED.is_short, \
                    updated_at = now() \
                 WHERE channel_videos.channel_id = EXCLUDED.channel_id",
            )
            .bind(&video.video_id)
            .bind(&channel.id)
            .bind(&video.title)
            .bind(video.duration_seconds)
            .bind(video.published_at)
            .bind(&video.published_text)
            .bind(Json(&video.thumbnails))
            .bind(video.view_count)
            .bind(video.is_live)
            .bind(video.is_upcoming)
            .bind(video.is_short)
            .bind(position)
            .execute(&mut *tx)
            .await?;
        }
        tx.commit().await?;
        Ok(())
    }

    async fn record_failure(&self, id: &str, error: &str) -> Result<(), Error> {
        sqlx::query("UPDATE channels SET checked_at = now(), last_error = $2 WHERE id = $1")
            .bind(id)
            .bind(error)
            .execute(&self.pool)
            .await?;
        Ok(())
    }

    async fn prune_unsubscribed(&self) -> Result<u64, Error> {
        let mut tx = self.pool.begin().await?;
        let deleted = sqlx::query(
            "DELETE FROM channel_videos v WHERE NOT EXISTS \
                (SELECT 1 FROM subscriptions s WHERE s.channel_id = v.channel_id)",
        )
        .execute(&mut *tx)
        .await?
        .rows_affected();
        // Forces a fresh fetch should someone subscribe again.
        sqlx::query(
            "UPDATE channels c SET fetched_at = NULL, checked_at = NULL \
             WHERE c.checked_at IS NOT NULL AND NOT EXISTS \
                (SELECT 1 FROM subscriptions s WHERE s.channel_id = c.id)",
        )
        .execute(&mut *tx)
        .await?;
        tx.commit().await?;
        Ok(deleted)
    }

    async fn status(&self) -> Result<FeedStatus, Error> {
        Ok(sqlx::query_as::<_, FeedStatus>(
            "SELECT count(*) AS channels, \
                count(*) FILTER (WHERE fetched_at IS NULL) AS pending, \
                count(*) FILTER (WHERE last_error IS NOT NULL) AS failing, \
                max(checked_at) AS last_checked_at \
             FROM channels c \
             WHERE EXISTS (SELECT 1 FROM subscriptions s WHERE s.channel_id = c.id)",
        )
        .fetch_one(&self.pool)
        .await?)
    }
}
