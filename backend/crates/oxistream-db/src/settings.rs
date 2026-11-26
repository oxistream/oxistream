use sqlx::PgPool;
use uuid::Uuid;

use crate::Error;

/// Per-user preferences. Users without a `user_settings` row have the defaults.
#[derive(Debug, Clone, Default, PartialEq, Eq, sqlx::FromRow)]
pub struct UserSettings {
    /// Watch progress is not recorded while set.
    pub history_paused: bool,
}

#[async_trait::async_trait]
pub trait UserSettingsRepository: Send + Sync {
    async fn get(&self, user_id: Uuid) -> Result<UserSettings, Error>;

    async fn set_history_paused(&self, user_id: Uuid, paused: bool) -> Result<UserSettings, Error>;
}

#[derive(Debug, Clone)]
pub struct PgUserSettingsRepository {
    pool: PgPool,
}

impl PgUserSettingsRepository {
    pub fn new(pool: PgPool) -> Self {
        Self { pool }
    }
}

#[async_trait::async_trait]
impl UserSettingsRepository for PgUserSettingsRepository {
    async fn get(&self, user_id: Uuid) -> Result<UserSettings, Error> {
        Ok(sqlx::query_as::<_, UserSettings>(
            "SELECT history_paused FROM user_settings WHERE user_id = $1",
        )
        .bind(user_id)
        .fetch_optional(&self.pool)
        .await?
        .unwrap_or_default())
    }

    async fn set_history_paused(&self, user_id: Uuid, paused: bool) -> Result<UserSettings, Error> {
        Ok(sqlx::query_as::<_, UserSettings>(
            "INSERT INTO user_settings (user_id, history_paused) VALUES ($1, $2) \
             ON CONFLICT (user_id) DO UPDATE SET \
                history_paused = EXCLUDED.history_paused, \
                updated_at = now() \
             RETURNING history_paused",
        )
        .bind(user_id)
        .bind(paused)
        .fetch_one(&self.pool)
        .await?)
    }
}
