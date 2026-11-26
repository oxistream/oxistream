use sqlx::postgres::PgRow;
use sqlx::types::Json;
use sqlx::{PgPool, Row};
use uuid::Uuid;

use crate::Error;

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
}

/// Fields to change; `None` keeps the stored value, `Some(None)` resets it.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
#[allow(clippy::option_option)]
pub struct SettingsUpdate {
    pub video_codecs: Option<Vec<String>>,
    pub max_quality: Option<Option<i32>>,
    pub default_quality: Option<Option<i32>>,
    pub home_sections: Option<Option<Vec<String>>>,
}

#[async_trait::async_trait]
pub trait UserSettingsRepository: Send + Sync {
    async fn get(&self, user_id: Uuid) -> Result<UserSettings, Error>;

    async fn set_history_paused(&self, user_id: Uuid, paused: bool) -> Result<UserSettings, Error>;

    async fn update(&self, user_id: Uuid, update: &SettingsUpdate) -> Result<UserSettings, Error>;
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
        "history_paused, video_codecs, max_quality, default_quality, home_sections"
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
                (user_id, video_codecs, max_quality, default_quality, home_sections) \
             VALUES ($1, COALESCE($2, '{}'), $4, $6, $8) \
             ON CONFLICT (user_id) DO UPDATE SET \
                video_codecs = COALESCE($2, user_settings.video_codecs), \
                max_quality = CASE WHEN $3 THEN $4 ELSE user_settings.max_quality END, \
                default_quality = CASE WHEN $5 THEN $6 ELSE user_settings.default_quality END, \
                home_sections = CASE WHEN $7 THEN $8 ELSE user_settings.home_sections END, \
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
        .fetch_one(&self.pool)
        .await?;
        Ok(settings_from_row(&row)?)
    }
}
