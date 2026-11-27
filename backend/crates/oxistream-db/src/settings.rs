use sqlx::postgres::PgRow;
use sqlx::types::Json;
use sqlx::{PgPool, Row};
use time::OffsetDateTime;
use uuid::Uuid;

use crate::{Error, TokenHash};

/// Per-user preferences. Users without a `user_settings` row have the defaults.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct UserSettings {
    /// Watch progress is not recorded while set.
    pub history_paused: bool,
    /// Video codecs by preference; empty lets the player choose.
    pub video_codecs: Vec<String>,
    /// Highest video height the player may pick.
    pub max_quality: Option<i32>,
    /// Video height to start with instead of adaptive selection.
    pub default_quality: Option<i32>,
    /// Enabled home page sections in order; `None` for the default layout.
    pub home_sections: Option<Vec<String>>,
    /// Preferred audio language, `interface` for the interface language; `None` for the original.
    pub audio_language: Option<String>,
}

/// Fields to change; `None` keeps the stored value, `Some(None)` resets it.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
#[allow(clippy::option_option)]
pub struct SettingsUpdate {
    pub video_codecs: Option<Vec<String>>,
    pub max_quality: Option<Option<i32>>,
    pub default_quality: Option<Option<i32>>,
    pub home_sections: Option<Option<Vec<String>>>,
    pub audio_language: Option<Option<String>>,
}

#[async_trait::async_trait]
pub trait UserSettingsRepository: Send + Sync {
    async fn get(&self, user_id: Uuid) -> Result<UserSettings, Error>;

    async fn set_history_paused(&self, user_id: Uuid, paused: bool) -> Result<UserSettings, Error>;

    async fn update(&self, user_id: Uuid, update: &SettingsUpdate) -> Result<UserSettings, Error>;

    /// When the user's feed token was created; `None` without one.
    async fn feed_token_created_at(&self, user_id: Uuid) -> Result<Option<OffsetDateTime>, Error>;

    /// Stores a new feed token, replacing any previous one; returns its creation time.
    async fn replace_feed_token(
        &self,
        user_id: Uuid,
        token_hash: &TokenHash,
    ) -> Result<OffsetDateTime, Error>;

    /// Returns whether a feed token existed.
    async fn delete_feed_token(&self, user_id: Uuid) -> Result<bool, Error>;

    /// The user owning the feed token, unless that user is disabled.
    async fn feed_token_user(&self, token_hash: &TokenHash) -> Result<Option<Uuid>, Error>;
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

macro_rules! settings_columns {
    () => {
        "history_paused, video_codecs, max_quality, default_quality, home_sections, audio_language"
    };
}

fn settings_from_row(row: &PgRow) -> Result<UserSettings, sqlx::Error> {
    let home_sections: Option<Json<Vec<String>>> = row.try_get("home_sections")?;
    Ok(UserSettings {
        history_paused: row.try_get("history_paused")?,
        video_codecs: row.try_get("video_codecs")?,
        max_quality: row.try_get("max_quality")?,
        default_quality: row.try_get("default_quality")?,
        home_sections: home_sections.map(|Json(sections)| sections),
        audio_language: row.try_get("audio_language")?,
    })
}

#[async_trait::async_trait]
impl UserSettingsRepository for PgUserSettingsRepository {
    async fn get(&self, user_id: Uuid) -> Result<UserSettings, Error> {
        let row = sqlx::query(concat!(
            "SELECT ",
            settings_columns!(),
            " FROM user_settings WHERE user_id = $1"
        ))
        .bind(user_id)
        .fetch_optional(&self.pool)
        .await?;
        Ok(row
            .as_ref()
            .map(settings_from_row)
            .transpose()?
            .unwrap_or_default())
    }

    async fn set_history_paused(&self, user_id: Uuid, paused: bool) -> Result<UserSettings, Error> {
        let row = sqlx::query(concat!(
            "INSERT INTO user_settings (user_id, history_paused) VALUES ($1, $2) \
             ON CONFLICT (user_id) DO UPDATE SET \
                history_paused = EXCLUDED.history_paused, \
                updated_at = now() \
             RETURNING ",
            settings_columns!()
        ))
        .bind(user_id)
        .bind(paused)
        .fetch_one(&self.pool)
        .await?;
        Ok(settings_from_row(&row)?)
    }

    async fn update(&self, user_id: Uuid, update: &SettingsUpdate) -> Result<UserSettings, Error> {
        // The boolean parameters tell "keep" apart from "reset to NULL".
        let row = sqlx::query(concat!(
            "INSERT INTO user_settings \
                (user_id, video_codecs, max_quality, default_quality, home_sections, \
                 audio_language) \
             VALUES ($1, COALESCE($2, '{}'), $4, $6, $8, $10) \
             ON CONFLICT (user_id) DO UPDATE SET \
                video_codecs = COALESCE($2, user_settings.video_codecs), \
                max_quality = CASE WHEN $3 THEN $4 ELSE user_settings.max_quality END, \
                default_quality = CASE WHEN $5 THEN $6 ELSE user_settings.default_quality END, \
                home_sections = CASE WHEN $7 THEN $8 ELSE user_settings.home_sections END, \
                audio_language = CASE WHEN $9 THEN $10 ELSE user_settings.audio_language END, \
                updated_at = now() \
             RETURNING ",
            settings_columns!()
        ))
        .bind(user_id)
        .bind(update.video_codecs.as_deref())
        .bind(update.max_quality.is_some())
        .bind(update.max_quality.flatten())
        .bind(update.default_quality.is_some())
        .bind(update.default_quality.flatten())
        .bind(update.home_sections.is_some())
        .bind(
            update
                .home_sections
                .as_ref()
                .and_then(|sections| sections.as_ref().map(Json)),
        )
        .bind(update.audio_language.is_some())
        .bind(update.audio_language.clone().flatten())
        .fetch_one(&self.pool)
        .await?;
        Ok(settings_from_row(&row)?)
    }

    async fn feed_token_created_at(&self, user_id: Uuid) -> Result<Option<OffsetDateTime>, Error> {
        Ok(
            sqlx::query_scalar("SELECT created_at FROM feed_tokens WHERE user_id = $1")
                .bind(user_id)
                .fetch_optional(&self.pool)
                .await?,
        )
    }

    async fn replace_feed_token(
        &self,
        user_id: Uuid,
        token_hash: &TokenHash,
    ) -> Result<OffsetDateTime, Error> {
        Ok(sqlx::query_scalar(
            "INSERT INTO feed_tokens (user_id, token_hash) VALUES ($1, $2) \
             ON CONFLICT (user_id) DO UPDATE SET \
                token_hash = EXCLUDED.token_hash, \
                created_at = now() \
             RETURNING created_at",
        )
        .bind(user_id)
        .bind(token_hash.as_slice())
        .fetch_one(&self.pool)
        .await?)
    }

    async fn delete_feed_token(&self, user_id: Uuid) -> Result<bool, Error> {
        let result = sqlx::query("DELETE FROM feed_tokens WHERE user_id = $1")
            .bind(user_id)
            .execute(&self.pool)
            .await?;
        Ok(result.rows_affected() > 0)
    }

    async fn feed_token_user(&self, token_hash: &TokenHash) -> Result<Option<Uuid>, Error> {
        Ok(sqlx::query_scalar(
            "SELECT t.user_id FROM feed_tokens t JOIN users u ON u.id = t.user_id \
             WHERE t.token_hash = $1 AND NOT u.disabled",
        )
        .bind(token_hash.as_slice())
        .fetch_optional(&self.pool)
        .await?)
    }
}
