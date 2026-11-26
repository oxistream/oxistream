use std::collections::HashSet;

use sqlx::PgPool;
use sqlx::types::Json;
use time::OffsetDateTime;
use uuid::Uuid;

use crate::{Error, Image};

/// A channel a user blocked, with the name and avatar it had then.
#[derive(Debug, Clone, PartialEq, Eq, sqlx::FromRow)]
pub struct ChannelBlock {
    pub channel_id: String,
    pub channel_name: String,
    #[sqlx(json)]
    pub avatar: Vec<Image>,
    pub created_at: OffsetDateTime,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct NewChannelBlock {
    pub channel_id: String,
    pub channel_name: String,
    pub avatar: Vec<Image>,
}

#[async_trait::async_trait]
pub trait ChannelBlockRepository: Send + Sync {
    /// Blocks a channel and ends a subscription to it, in one transaction. Blocking again keeps
    /// the original date. Returns the block and whether it is new.
    async fn block(
        &self,
        user_id: Uuid,
        block: &NewChannelBlock,
    ) -> Result<(ChannelBlock, bool), Error>;

    /// Returns whether a block existed.
    async fn unblock(&self, user_id: Uuid, channel_id: &str) -> Result<bool, Error>;

    async fn get(&self, user_id: Uuid, channel_id: &str) -> Result<Option<ChannelBlock>, Error>;

    /// All blocks of a user, most recent first.
    async fn list(&self, user_id: Uuid) -> Result<Vec<ChannelBlock>, Error>;

    /// IDs of all channels the user blocked, for filtering listings.
    async fn blocked_ids(&self, user_id: Uuid) -> Result<HashSet<String>, Error>;
}

#[derive(Debug, Clone)]
pub struct PgChannelBlockRepository {
    pool: PgPool,
}

impl PgChannelBlockRepository {
    pub fn new(pool: PgPool) -> Self {
        Self { pool }
    }
}

#[derive(sqlx::FromRow)]
struct StoredBlock {
    #[sqlx(flatten)]
    block: ChannelBlock,
    created: bool,
}

const BLOCK_COLUMNS: &str = "channel_id, channel_name, avatar, created_at";

#[async_trait::async_trait]
impl ChannelBlockRepository for PgChannelBlockRepository {
    async fn block(
        &self,
        user_id: Uuid,
        block: &NewChannelBlock,
    ) -> Result<(ChannelBlock, bool), Error> {
        let mut tx = self.pool.begin().await?;
        // `xmax = 0` only for a freshly inserted row.
        let stored = sqlx::query_as::<_, StoredBlock>(
            "INSERT INTO channel_blocks (user_id, channel_id, channel_name, avatar) \
             VALUES ($1, $2, $3, $4) \
             ON CONFLICT (user_id, channel_id) DO UPDATE SET \
                channel_name = EXCLUDED.channel_name, avatar = EXCLUDED.avatar \
             RETURNING channel_id, channel_name, avatar, created_at, (xmax = 0) AS created",
        )
        .bind(user_id)
        .bind(&block.channel_id)
        .bind(&block.channel_name)
        .bind(Json(&block.avatar))
        .fetch_one(&mut *tx)
        .await?;
        sqlx::query("DELETE FROM subscriptions WHERE user_id = $1 AND channel_id = $2")
            .bind(user_id)
            .bind(&block.channel_id)
            .execute(&mut *tx)
            .await?;
        tx.commit().await?;
        Ok((stored.block, stored.created))
    }

    async fn unblock(&self, user_id: Uuid, channel_id: &str) -> Result<bool, Error> {
        Ok(
            sqlx::query("DELETE FROM channel_blocks WHERE user_id = $1 AND channel_id = $2")
                .bind(user_id)
                .bind(channel_id)
                .execute(&self.pool)
                .await?
                .rows_affected()
                > 0,
        )
    }

    async fn get(&self, user_id: Uuid, channel_id: &str) -> Result<Option<ChannelBlock>, Error> {
        Ok(
            sqlx::query_as::<_, ChannelBlock>(sqlx::AssertSqlSafe(format!(
                "SELECT {BLOCK_COLUMNS} FROM channel_blocks WHERE user_id = $1 AND channel_id = $2"
            )))
            .bind(user_id)
            .bind(channel_id)
            .fetch_optional(&self.pool)
            .await?,
        )
    }

    async fn list(&self, user_id: Uuid) -> Result<Vec<ChannelBlock>, Error> {
        Ok(
            sqlx::query_as::<_, ChannelBlock>(sqlx::AssertSqlSafe(format!(
                "SELECT {BLOCK_COLUMNS} FROM channel_blocks WHERE user_id = $1 \
             ORDER BY created_at DESC, channel_id"
            )))
            .bind(user_id)
            .fetch_all(&self.pool)
            .await?,
        )
    }

    async fn blocked_ids(&self, user_id: Uuid) -> Result<HashSet<String>, Error> {
        let ids: Vec<String> =
            sqlx::query_scalar("SELECT channel_id FROM channel_blocks WHERE user_id = $1")
                .bind(user_id)
                .fetch_all(&self.pool)
                .await?;
        Ok(ids.into_iter().collect())
    }
}
