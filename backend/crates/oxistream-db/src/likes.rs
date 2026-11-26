use sqlx::PgPool;
use time::OffsetDateTime;
use uuid::Uuid;

use crate::{Error, Video};

/// A liked video with its cached metadata.
#[derive(Debug, Clone, PartialEq, Eq, sqlx::FromRow)]
pub struct LikedVideo {
    pub liked_at: OffsetDateTime,
    #[sqlx(flatten)]
    pub video: Video,
}

impl LikedVideo {
    /// The cursor that continues the list after this video.
    pub fn cursor(&self) -> LikeCursor {
        LikeCursor {
            liked_at: self.liked_at,
            video_id: self.video.id.clone(),
        }
    }
}

/// Position in the list of liked videos; the next page starts after it.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct LikeCursor {
    pub liked_at: OffsetDateTime,
    pub video_id: String,
}

#[async_trait::async_trait]
pub trait LikeRepository: Send + Sync {
    /// Likes a cached video. Returns when it was liked and whether that happened just now.
    async fn like(&self, user_id: Uuid, video_id: &str) -> Result<(OffsetDateTime, bool), Error>;

    /// Returns whether the video was liked.
    async fn unlike(&self, user_id: Uuid, video_id: &str) -> Result<bool, Error>;

    /// When the user liked the video, if they did.
    async fn liked_at(
        &self,
        user_id: Uuid,
        video_id: &str,
    ) -> Result<Option<OffsetDateTime>, Error>;

    /// Liked videos, most recently liked first, starting after `after`.
    async fn list(
        &self,
        user_id: Uuid,
        after: Option<&LikeCursor>,
        limit: u32,
    ) -> Result<Vec<LikedVideo>, Error>;

    async fn count(&self, user_id: Uuid) -> Result<i64, Error>;
}

#[derive(Debug, Clone)]
pub struct PgLikeRepository {
    pool: PgPool,
}

impl PgLikeRepository {
    pub fn new(pool: PgPool) -> Self {
        Self { pool }
    }
}

#[async_trait::async_trait]
impl LikeRepository for PgLikeRepository {
    async fn like(&self, user_id: Uuid, video_id: &str) -> Result<(OffsetDateTime, bool), Error> {
        let created: Option<OffsetDateTime> = sqlx::query_scalar(
            "INSERT INTO video_likes (user_id, video_id) VALUES ($1, $2) \
             ON CONFLICT (user_id, video_id) DO NOTHING RETURNING created_at",
        )
        .bind(user_id)
        .bind(video_id)
        .fetch_optional(&self.pool)
        .await?;
        if let Some(at) = created {
            return Ok((at, true));
        }
        let at = sqlx::query_scalar(
            "SELECT created_at FROM video_likes WHERE user_id = $1 AND video_id = $2",
        )
        .bind(user_id)
        .bind(video_id)
        .fetch_one(&self.pool)
        .await?;
        Ok((at, false))
    }

    async fn unlike(&self, user_id: Uuid, video_id: &str) -> Result<bool, Error> {
        Ok(
            sqlx::query("DELETE FROM video_likes WHERE user_id = $1 AND video_id = $2")
                .bind(user_id)
                .bind(video_id)
                .execute(&self.pool)
                .await?
                .rows_affected()
                > 0,
        )
    }

    async fn liked_at(
        &self,
        user_id: Uuid,
        video_id: &str,
    ) -> Result<Option<OffsetDateTime>, Error> {
        Ok(sqlx::query_scalar(
            "SELECT created_at FROM video_likes WHERE user_id = $1 AND video_id = $2",
        )
        .bind(user_id)
        .bind(video_id)
        .fetch_optional(&self.pool)
        .await?)
    }

    async fn list(
        &self,
        user_id: Uuid,
        after: Option<&LikeCursor>,
        limit: u32,
    ) -> Result<Vec<LikedVideo>, Error> {
        Ok(sqlx::query_as::<_, LikedVideo>(
            "SELECT l.created_at AS liked_at, v.id, v.title, v.channel_id, v.channel_name, \
                v.duration_seconds, v.published_at, v.thumbnail_url, v.updated_at \
             FROM video_likes l JOIN videos v ON v.id = l.video_id \
             WHERE l.user_id = $1 \
                AND ($2::timestamptz IS NULL OR (l.created_at, l.video_id) < ($2, $3)) \
             ORDER BY l.created_at DESC, l.video_id DESC \
             LIMIT $4",
        )
        .bind(user_id)
        .bind(after.map(|c| c.liked_at))
        .bind(after.map(|c| c.video_id.as_str()))
        .bind(i64::from(limit))
        .fetch_all(&self.pool)
        .await?)
    }

    async fn count(&self, user_id: Uuid) -> Result<i64, Error> {
        Ok(
            sqlx::query_scalar("SELECT count(*) FROM video_likes WHERE user_id = $1")
                .bind(user_id)
                .fetch_one(&self.pool)
                .await?,
        )
    }
}
