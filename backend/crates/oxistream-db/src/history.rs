use sqlx::PgPool;
use time::OffsetDateTime;
use uuid::Uuid;

use crate::{Error, Image};

/// How far a user got into a video.
#[derive(Debug, Clone, PartialEq, sqlx::FromRow)]
pub struct WatchProgress {
    pub video_id: String,
    pub position_seconds: f64,
    pub duration_seconds: f64,
    pub finished: bool,
    pub updated_at: OffsetDateTime,
}

/// A progress report from the player.
#[derive(Debug, Clone, PartialEq)]
pub struct ProgressUpdate {
    pub video_id: String,
    pub position_seconds: f64,
    pub duration_seconds: f64,
    pub finished: bool,
}

/// A watch history entry with the cached video metadata, if any.
#[derive(Debug, Clone, PartialEq, sqlx::FromRow)]
pub struct HistoryEntry {
    #[sqlx(flatten)]
    pub progress: WatchProgress,
    /// `None` while the video's metadata is not cached.
    pub title: Option<String>,
    pub channel_id: Option<String>,
    pub channel_name: Option<String>,
    /// Known when someone subscribes to the channel.
    #[sqlx(json)]
    pub channel_avatar: Vec<Image>,
    pub channel_verified: bool,
    pub published_at: Option<OffsetDateTime>,
}

impl HistoryEntry {
    /// The cursor that continues the history after this entry.
    pub fn cursor(&self) -> HistoryCursor {
        HistoryCursor {
            updated_at: self.progress.updated_at,
            video_id: self.progress.video_id.clone(),
        }
    }
}

/// Position in a history listing; the next page starts after it.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct HistoryCursor {
    pub updated_at: OffsetDateTime,
    pub video_id: String,
}

#[async_trait::async_trait]
pub trait HistoryRepository: Send + Sync {
    /// Stores the latest progress of a video. Returns `false`, storing nothing, while the
    /// user paused their history or for a video known to be a short.
    async fn record(&self, user_id: Uuid, update: &ProgressUpdate) -> Result<bool, Error>;

    /// Progress of the given videos; videos never watched are left out.
    async fn progress(
        &self,
        user_id: Uuid,
        video_ids: &[String],
    ) -> Result<Vec<WatchProgress>, Error>;

    /// Watched videos, most recently watched first, starting after `after`.
    async fn list(
        &self,
        user_id: Uuid,
        after: Option<&HistoryCursor>,
        limit: u32,
    ) -> Result<Vec<HistoryEntry>, Error>;

    /// Unfinished videos watched for at least `min_position` seconds, most recent first.
    /// Videos known to be shorts are left out.
    async fn unfinished(
        &self,
        user_id: Uuid,
        min_position: f64,
        limit: u32,
    ) -> Result<Vec<HistoryEntry>, Error>;

    /// Hides a video from [`Self::unfinished`] until its next progress report; the history
    /// entry and resume point stay. Returns whether an entry existed.
    async fn dismiss_continue(&self, user_id: Uuid, video_id: &str) -> Result<bool, Error>;

    /// Returns whether an entry existed.
    async fn delete(&self, user_id: Uuid, video_id: &str) -> Result<bool, Error>;

    /// Returns the number of deleted entries.
    async fn clear(&self, user_id: Uuid) -> Result<u64, Error>;
}

#[derive(Debug, Clone)]
pub struct PgHistoryRepository {
    pool: PgPool,
}

impl PgHistoryRepository {
    pub fn new(pool: PgPool) -> Self {
        Self { pool }
    }
}

const ENTRY_QUERY: &str = "SELECT p.video_id, p.position_seconds, p.duration_seconds, \
        p.finished, p.updated_at, v.title, v.channel_id, \
        COALESCE(c.name, v.channel_name) AS channel_name, \
        COALESCE(c.avatar, '[]') AS channel_avatar, \
        COALESCE(c.verified, false) AS channel_verified, v.published_at \
     FROM watch_progress p \
     LEFT JOIN videos v ON v.id = p.video_id \
     LEFT JOIN channels c ON c.id = v.channel_id";

#[async_trait::async_trait]
impl HistoryRepository for PgHistoryRepository {
    async fn record(&self, user_id: Uuid, update: &ProgressUpdate) -> Result<bool, Error> {
        Ok(sqlx::query(
            "INSERT INTO watch_progress \
                (user_id, video_id, position_seconds, duration_seconds, finished) \
             SELECT $1, $2, $3, $4, $5 \
             WHERE NOT EXISTS (SELECT 1 FROM user_settings \
                WHERE user_id = $1 AND history_paused) \
                AND NOT EXISTS (SELECT 1 FROM channel_videos \
                WHERE video_id = $2 AND is_short) \
             ON CONFLICT (user_id, video_id) DO UPDATE SET \
                position_seconds = EXCLUDED.position_seconds, \
                duration_seconds = EXCLUDED.duration_seconds, \
                finished = EXCLUDED.finished, \
                continue_dismissed = false, \
                updated_at = now()",
        )
        .bind(user_id)
        .bind(&update.video_id)
        .bind(update.position_seconds)
        .bind(update.duration_seconds)
        .bind(update.finished)
        .execute(&self.pool)
        .await?
        .rows_affected()
            > 0)
    }

    async fn progress(
        &self,
        user_id: Uuid,
        video_ids: &[String],
    ) -> Result<Vec<WatchProgress>, Error> {
        if video_ids.is_empty() {
            return Ok(Vec::new());
        }
        Ok(sqlx::query_as::<_, WatchProgress>(
            "SELECT video_id, position_seconds, duration_seconds, finished, updated_at \
             FROM watch_progress WHERE user_id = $1 AND video_id = ANY($2)",
        )
        .bind(user_id)
        .bind(video_ids)
        .fetch_all(&self.pool)
        .await?)
    }

    async fn list(
        &self,
        user_id: Uuid,
        after: Option<&HistoryCursor>,
        limit: u32,
    ) -> Result<Vec<HistoryEntry>, Error> {
        Ok(
            sqlx::query_as::<_, HistoryEntry>(sqlx::AssertSqlSafe(format!(
                "{ENTRY_QUERY} \
             WHERE p.user_id = $1 \
                AND ($2::timestamptz IS NULL OR (p.updated_at, p.video_id) < ($2, $3)) \
             ORDER BY p.updated_at DESC, p.video_id DESC \
             LIMIT $4"
            )))
            .bind(user_id)
            .bind(after.map(|c| c.updated_at))
            .bind(after.map(|c| c.video_id.as_str()))
            .bind(i64::from(limit))
            .fetch_all(&self.pool)
            .await?,
        )
    }

    async fn unfinished(
        &self,
        user_id: Uuid,
        min_position: f64,
        limit: u32,
    ) -> Result<Vec<HistoryEntry>, Error> {
        Ok(
            sqlx::query_as::<_, HistoryEntry>(sqlx::AssertSqlSafe(format!(
                "{ENTRY_QUERY} \
             WHERE p.user_id = $1 AND NOT p.finished AND NOT p.continue_dismissed \
                AND p.position_seconds >= $2 \
                AND NOT EXISTS (SELECT 1 FROM channel_videos cv \
                WHERE cv.video_id = p.video_id AND cv.is_short) \
             ORDER BY p.updated_at DESC, p.video_id DESC \
             LIMIT $3"
            )))
            .bind(user_id)
            .bind(min_position)
            .bind(i64::from(limit))
            .fetch_all(&self.pool)
            .await?,
        )
    }

    async fn dismiss_continue(&self, user_id: Uuid, video_id: &str) -> Result<bool, Error> {
        Ok(sqlx::query(
            "UPDATE watch_progress SET continue_dismissed = true \
             WHERE user_id = $1 AND video_id = $2",
        )
        .bind(user_id)
        .bind(video_id)
        .execute(&self.pool)
        .await?
        .rows_affected()
            > 0)
    }

    async fn delete(&self, user_id: Uuid, video_id: &str) -> Result<bool, Error> {
        Ok(
            sqlx::query("DELETE FROM watch_progress WHERE user_id = $1 AND video_id = $2")
                .bind(user_id)
                .bind(video_id)
                .execute(&self.pool)
                .await?
                .rows_affected()
                > 0,
        )
    }

    async fn clear(&self, user_id: Uuid) -> Result<u64, Error> {
        Ok(sqlx::query("DELETE FROM watch_progress WHERE user_id = $1")
            .bind(user_id)
            .execute(&self.pool)
            .await?
            .rows_affected())
    }
}
