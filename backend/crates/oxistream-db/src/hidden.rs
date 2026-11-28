use std::collections::HashSet;

use sqlx::PgPool;
use uuid::Uuid;

use crate::Error;

/// Videos a user is not interested in.
#[async_trait::async_trait]
pub trait HiddenVideoRepository: Send + Sync {
    /// Returns whether the video was not hidden before.
    async fn hide(&self, user_id: Uuid, video_id: &str) -> Result<bool, Error>;

    /// Returns whether the video was hidden.
    async fn unhide(&self, user_id: Uuid, video_id: &str) -> Result<bool, Error>;

    /// IDs of all hidden videos, for filtering listings.
    async fn hidden_ids(&self, user_id: Uuid) -> Result<HashSet<String>, Error>;
}

#[derive(Debug, Clone)]
pub struct PgHiddenVideoRepository {
    pool: PgPool,
}

impl PgHiddenVideoRepository {
    pub fn new(pool: PgPool) -> Self {
        Self { pool }
    }
}

#[async_trait::async_trait]
impl HiddenVideoRepository for PgHiddenVideoRepository {
    async fn hide(&self, user_id: Uuid, video_id: &str) -> Result<bool, Error> {
        Ok(sqlx::query(
            "INSERT INTO hidden_videos (user_id, video_id) VALUES ($1, $2) \
             ON CONFLICT DO NOTHING",
        )
        .bind(user_id)
        .bind(video_id)
        .execute(&self.pool)
        .await?
        .rows_affected()
            > 0)
    }

    async fn unhide(&self, user_id: Uuid, video_id: &str) -> Result<bool, Error> {
        Ok(
            sqlx::query("DELETE FROM hidden_videos WHERE user_id = $1 AND video_id = $2")
                .bind(user_id)
                .bind(video_id)
                .execute(&self.pool)
                .await?
                .rows_affected()
                > 0,
        )
    }

    async fn hidden_ids(&self, user_id: Uuid) -> Result<HashSet<String>, Error> {
        let ids: Vec<String> =
            sqlx::query_scalar("SELECT video_id FROM hidden_videos WHERE user_id = $1")
                .bind(user_id)
                .fetch_all(&self.pool)
                .await?;
        Ok(ids.into_iter().collect())
    }
}
