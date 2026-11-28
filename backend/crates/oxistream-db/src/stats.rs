use sqlx::PgPool;

use crate::{Error, LockKey};

/// Row counts for the admin status page.
#[derive(Debug, Clone, Copy, PartialEq, Eq, sqlx::FromRow)]
pub struct InstanceCounts {
    pub users: i64,
    /// Admins that are not disabled.
    pub admins: i64,
    pub disabled_users: i64,
    pub subscriptions: i64,
    pub playlists: i64,
    /// Cached video metadata.
    pub videos: i64,
}

#[async_trait::async_trait]
pub trait StatsRepository: Send + Sync {
    async fn counts(&self) -> Result<InstanceCounts, Error>;

    /// The Postgres `server_version`.
    async fn server_version(&self) -> Result<String, Error>;

    /// Whether any session currently holds the advisory lock.
    async fn lock_held(&self, key: LockKey) -> Result<bool, Error>;
}

#[derive(Debug, Clone)]
pub struct PgStatsRepository {
    pool: PgPool,
}

impl PgStatsRepository {
    pub fn new(pool: PgPool) -> Self {
        Self { pool }
    }
}

#[async_trait::async_trait]
impl StatsRepository for PgStatsRepository {
    async fn counts(&self) -> Result<InstanceCounts, Error> {
        Ok(sqlx::query_as::<_, InstanceCounts>(
            "SELECT \
                (SELECT count(*) FROM users) AS users, \
                (SELECT count(*) FROM users WHERE role = 'admin' AND NOT disabled) AS admins, \
                (SELECT count(*) FROM users WHERE disabled) AS disabled_users, \
                (SELECT count(*) FROM subscriptions) AS subscriptions, \
                (SELECT count(*) FROM playlists) AS playlists, \
                (SELECT count(*) FROM videos) AS videos",
        )
        .fetch_one(&self.pool)
        .await?)
    }

    async fn server_version(&self) -> Result<String, Error> {
        Ok(
            sqlx::query_scalar("SELECT current_setting('server_version')")
                .fetch_one(&self.pool)
                .await?,
        )
    }

    async fn lock_held(&self, key: LockKey) -> Result<bool, Error> {
        // A bigint advisory key shows up split into classid (high) and objid (low 32 bits).
        Ok(sqlx::query_scalar(
            "SELECT EXISTS (SELECT 1 FROM pg_locks WHERE locktype = 'advisory' AND granted \
                AND objsubid = 1 \
                AND database = (SELECT oid FROM pg_database WHERE datname = current_database()) \
                AND ((classid::int8 << 32) | objid::int8) = $1)",
        )
        .bind(key as i64)
        .fetch_one(&self.pool)
        .await?)
    }
}
