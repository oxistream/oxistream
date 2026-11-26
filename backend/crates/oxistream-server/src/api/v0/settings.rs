use std::collections::{BTreeMap, HashSet};

use axum::Json;
use axum::extract::State;
use axum::extract::rejection::JsonRejection;
use axum::http::StatusCode;
use oxistream_db::{SettingsUpdate, UserSettings};
use serde::{Deserialize, Deserializer, Serialize};
use utoipa::ToSchema;
use utoipa_axum::router::OpenApiRouter;
use utoipa_axum::routes;

use crate::AppState;
use crate::api::error::{ApiError, ErrorBody, FieldError};
use crate::auth::CurrentUser;

/// Video heights offered as quality limits.
pub const QUALITIES: [u32; 9] = [144, 240, 360, 480, 720, 1080, 1440, 2160, 4320];
const MAX_HOME_SECTIONS: usize = 32;
const MAX_SECTION_KEY_LENGTH: usize = 64;

pub fn router() -> OpenApiRouter<AppState> {
    OpenApiRouter::new().routes(routes!(get_settings, update_settings))
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize, ToSchema)]
#[serde(rename_all = "snake_case")]
pub enum VideoCodec {
    Av1,
    Vp9,
    H264,
}

impl VideoCodec {
    fn as_str(self) -> &'static str {
        match self {
            Self::Av1 => "av1",
            Self::Vp9 => "vp9",
            Self::H264 => "h264",
        }
    }

    fn parse(value: &str) -> Option<Self> {
        match value {
            "av1" => Some(Self::Av1),
            "vp9" => Some(Self::Vp9),
            "h264" => Some(Self::H264),
            _ => None,
        }
    }
}

/// The signed-in user's preferences. The watch history pause lives at `/history/settings`.
#[derive(Debug, Serialize, ToSchema)]
pub struct Settings {
    /// Video codecs by preference, most preferred first; empty lets the player choose.
    pub video_codecs: Vec<VideoCodec>,
    /// Highest video height the player may pick; `null` for no limit.
    #[schema(required = true)]
    pub max_quality: Option<u32>,
    /// Video height to start with; `null` for adaptive quality.
    #[schema(required = true)]
    pub default_quality: Option<u32>,
    /// Enabled home page section keys in order; `null` for the default layout. Clients ignore
    /// keys they do not know.
    #[schema(required = true)]
    pub home_sections: Option<Vec<String>>,
}

/// Fields to change. Absent fields are kept; `null` resets a field to its default.
#[derive(Debug, Default, Deserialize, ToSchema)]
#[serde(deny_unknown_fields)]
#[allow(clippy::option_option)]
pub struct SettingsPatch {
    /// No duplicates.
    #[serde(default)]
    #[schema(nullable = false)]
    pub video_codecs: Option<Vec<VideoCodec>>,
    /// One of 144, 240, 360, 480, 720, 1080, 1440, 2160, 4320.
    #[serde(default, deserialize_with = "nullable")]
    #[schema(value_type = Option<u32>)]
    pub max_quality: Option<Option<u32>>,
    /// One of 144, 240, 360, 480, 720, 1080, 1440, 2160, 4320.
    #[serde(default, deserialize_with = "nullable")]
    #[schema(value_type = Option<u32>)]
    pub default_quality: Option<Option<u32>>,
    /// At most 32 distinct keys of lowercase letters, digits and dashes.
    #[serde(default, deserialize_with = "nullable")]
    #[schema(value_type = Option<Vec<String>>)]
    pub home_sections: Option<Option<Vec<String>>>,
}

/// Tells a field set to `null` (`Some(None)`) apart from an absent one (`None`).
#[allow(clippy::option_option)]
fn nullable<'de, T: Deserialize<'de>, D: Deserializer<'de>>(
    deserializer: D,
) -> Result<Option<Option<T>>, D::Error> {
    Option::<T>::deserialize(deserializer).map(Some)
}

fn quality(value: Option<i32>) -> Option<u32> {
    value.and_then(|v| u32::try_from(v).ok())
}

impl From<UserSettings> for Settings {
    fn from(settings: UserSettings) -> Self {
        Self {
            video_codecs: settings
                .video_codecs
                .iter()
                .filter_map(|codec| VideoCodec::parse(codec))
                .collect(),
            max_quality: quality(settings.max_quality),
            default_quality: quality(settings.default_quality),
            home_sections: settings.home_sections,
        }
    }
}

fn valid_section_key(key: &str) -> bool {
    !key.is_empty()
        && key.len() <= MAX_SECTION_KEY_LENGTH
        && key
            .bytes()
            .all(|b| b.is_ascii_lowercase() || b.is_ascii_digit() || b == b'-')
}

fn distinct<T: Eq + std::hash::Hash>(items: &[T]) -> bool {
    let mut seen = HashSet::new();
    items.iter().all(|item| seen.insert(item))
}

impl SettingsPatch {
    fn validate(self) -> Result<SettingsUpdate, ApiError> {
        let mut fields = BTreeMap::new();
        if let Some(codecs) = &self.video_codecs
            && !distinct(codecs)
        {
            fields.insert("video_codecs".to_owned(), FieldError::Invalid);
        }
        for (name, value) in [
            ("max_quality", self.max_quality),
            ("default_quality", self.default_quality),
        ] {
            if let Some(Some(height)) = value
                && !QUALITIES.contains(&height)
            {
                fields.insert(name.to_owned(), FieldError::Invalid);
            }
        }
        if let Some(Some(sections)) = &self.home_sections
            && (sections.len() > MAX_HOME_SECTIONS
                || !sections.iter().all(|key| valid_section_key(key))
                || !distinct(sections))
        {
            fields.insert("home_sections".to_owned(), FieldError::Invalid);
        }
        if !fields.is_empty() {
            return Err(ApiError::validation(
                StatusCode::UNPROCESSABLE_ENTITY,
                fields,
            ));
        }
        // Validated against `QUALITIES`, so the heights fit.
        #[allow(clippy::cast_possible_wrap)]
        let height = |value: Option<Option<u32>>| value.map(|v| v.map(|h| h as i32));
        Ok(SettingsUpdate {
            video_codecs: self.video_codecs.map(|codecs| {
                codecs
                    .into_iter()
                    .map(|codec| codec.as_str().to_owned())
                    .collect()
            }),
            max_quality: height(self.max_quality),
            default_quality: height(self.default_quality),
            home_sections: self.home_sections,
        })
    }
}

/// The signed-in user's preferences.
#[utoipa::path(
    get,
    path = "/settings",
    tag = "settings",
    security(("session" = [])),
    responses(
        (status = OK, body = Settings),
        (status = UNAUTHORIZED, description = "Not signed in", body = ErrorBody),
    )
)]
async fn get_settings(
    State(state): State<AppState>,
    CurrentUser(user): CurrentUser,
) -> Result<Json<Settings>, ApiError> {
    Ok(Json(state.settings.get(user.id).await?.into()))
}

/// Change some of the signed-in user's preferences.
#[utoipa::path(
    patch,
    path = "/settings",
    tag = "settings",
    security(("session" = [])),
    request_body = SettingsPatch,
    responses(
        (status = OK, description = "All preferences after the change", body = Settings),
        (status = BAD_REQUEST, description = "Malformed body or unknown field", body = ErrorBody),
        (status = UNAUTHORIZED, description = "Not signed in", body = ErrorBody),
        (status = FORBIDDEN, description = "Foreign origin", body = ErrorBody),
        (status = UNPROCESSABLE_ENTITY, description = "Invalid values, see `fields`", body = ErrorBody),
    )
)]
async fn update_settings(
    State(state): State<AppState>,
    CurrentUser(user): CurrentUser,
    body: Result<Json<SettingsPatch>, JsonRejection>,
) -> Result<Json<Settings>, ApiError> {
    let Json(patch) = body?;
    let update = patch.validate()?;
    Ok(Json(state.settings.update(user.id, &update).await?.into()))
}

#[cfg(test)]
mod tests {
    use super::*;

    fn patch(json: serde_json::Value) -> SettingsPatch {
        serde_json::from_value(json).unwrap()
    }

    #[test]
    fn tells_null_from_absent() {
        let p = patch(serde_json::json!({ "max_quality": null }));
        assert_eq!(p.max_quality, Some(None));
        assert_eq!(p.default_quality, None);
        assert_eq!(p.home_sections, None);
    }

    #[test]
    fn rejects_invalid_values() {
        let err = patch(serde_json::json!({
            "video_codecs": ["av1", "av1"],
            "max_quality": 1000,
            "default_quality": 720,
            "home_sections": ["Bad Key"],
        }))
        .validate()
        .unwrap_err();
        let fields = err.body.fields.unwrap();
        assert_eq!(
            fields.keys().collect::<Vec<_>>(),
            ["home_sections", "max_quality", "video_codecs"]
        );
        assert!(
            serde_json::from_value::<SettingsPatch>(serde_json::json!({ "codec": "av1" })).is_err()
        );
    }

    #[test]
    fn converts_valid_patch() {
        let update = patch(serde_json::json!({
            "video_codecs": ["vp9", "h264"],
            "default_quality": 1080,
            "home_sections": ["subscriptions", "trending-news"],
        }))
        .validate()
        .unwrap();
        assert_eq!(update.video_codecs, Some(vec!["vp9".into(), "h264".into()]));
        assert_eq!(update.max_quality, None);
        assert_eq!(update.default_quality, Some(Some(1080)));
        assert_eq!(
            update.home_sections,
            Some(Some(vec!["subscriptions".into(), "trending-news".into()]))
        );
    }
}
