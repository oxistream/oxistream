use sqlx::PgPool;
use time::OffsetDateTime;

use crate::Error;

/// Cached metadata of a YouTube video.
#[derive(Debug, Clone, PartialEq, Eq, sqlx::FromRow)]
pub struct Video {
    pub id: String,
    pub title: String,
    pub channel_id: String,
    pub channel_name: String,
    pub duration_seconds: Option<i32>,
    pub published_at: Option<OffsetDateTime>,
    pub thumbnail_url: Option<String>,
    pub updated_at: OffsetDateTime,
}

/// Fields written by [`VideoRepository::upsert`]; `updated_at` is set by the database.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct VideoUpsert {
    pub id: String,
    pub title: String,
    pub channel_id: String,
    pub channel_name: String,
    pub duration_seconds: Option<i32>,
    pub published_at: Option<OffsetDateTime>,
    pub thumbnail_url: Option<String>,
}

#[async_trait::async_trait]
pub trait VideoRepository: Send + Sync {
    /// Inserts or updates a video. Unknown (`None`) fields keep their stored value.
    async fn upsert(&self, video: &VideoUpsert) -> Result<(), Error>;

    async fn get(&self, id: &str) -> Result<Option<Video>, Error>;
}

#[derive(Debug, Clone)]
pub struct PgVideoRepository {
    pool: PgPool,
}

impl PgVideoRepository {
    pub fn new(pool: PgPool) -> Self {
        Self { pool }
    }
}

#[async_trait::async_trait]
impl VideoRepository for PgVideoRepository {
    async fn upsert(&self, video: &VideoUpsert) -> Result<(), Error> {
        sqlx::query(
            "INSERT INTO videos \
                (id, title, channel_id, channel_name, duration_seconds, published_at, thumbnail_url) \
             VALUES ($1, $2, $3, $4, $5, $6, $7) \
             ON CONFLICT (id) DO UPDATE SET \
                title = EXCLUDED.title, \
                channel_id = EXCLUDED.channel_id, \
                channel_name = EXCLUDED.channel_name, \
                duration_seconds = COALESCE(EXCLUDED.duration_seconds, videos.duration_seconds), \
                published_at = COALESCE(EXCLUDED.published_at, videos.published_at), \
                thumbnail_url = COALESCE(EXCLUDED.thumbnail_url, videos.thumbnail_url), \
                updated_at = now()",
        )
        .bind(&video.id)
        .bind(&video.title)
        .bind(&video.channel_id)
        .bind(&video.channel_name)
        .bind(video.duration_seconds)
        .bind(video.published_at)
        .bind(&video.thumbnail_url)
        .execute(&self.pool)
        .await?;
        Ok(())
    }

    async fn get(&self, id: &str) -> Result<Option<Video>, Error> {
        Ok(sqlx::query_as::<_, Video>(
            "SELECT id, title, channel_id, channel_name, duration_seconds, published_at, \
                thumbnail_url, updated_at \
             FROM videos WHERE id = $1",
        )
        .bind(id)
        .fetch_optional(&self.pool)
        .await?)
    }
}
