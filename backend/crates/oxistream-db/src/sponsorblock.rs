use sqlx::PgPool;
use sqlx::types::JsonValue;
use uuid::Uuid;

use crate::Error;

/// SponsorBlock preferences as a JSON document; its shape is up to the server.
#[async_trait::async_trait]
pub trait SponsorBlockSettingsRepository: Send + Sync {
    /// `None` when the user never saved preferences.
    async fn get(&self, user_id: Uuid) -> Result<Option<JsonValue>, Error>;

    async fn set(&self, user_id: Uuid, settings: &JsonValue) -> Result<(), Error>;
}

#[derive(Debug, Clone)]
pub struct PgSponsorBlockSettingsRepository {
    pool: PgPool,
}

impl PgSponsorBlockSettingsRepository {
    pub fn new(pool: PgPool) -> Self {
        Self { pool }
    }
}

#[async_trait::async_trait]
impl SponsorBlockSettingsRepository for PgSponsorBlockSettingsRepository {
    async fn get(&self, user_id: Uuid) -> Result<Option<JsonValue>, Error> {
        Ok(sqlx::query_scalar::<_, JsonValue>(
            "SELECT settings FROM user_sponsorblock_settings WHERE user_id = $1",
        )
        .bind(user_id)
        .fetch_optional(&self.pool)
        .await?)
    }

    async fn set(&self, user_id: Uuid, settings: &JsonValue) -> Result<(), Error> {
        sqlx::query(
            "INSERT INTO user_sponsorblock_settings (user_id, settings) VALUES ($1, $2) \
             ON CONFLICT (user_id) DO UPDATE SET \
                settings = EXCLUDED.settings, \
                updated_at = now()",
        )
        .bind(user_id)
        .bind(settings)
        .execute(&self.pool)
        .await?;
        Ok(())
    }
}
