use axum::Json;
use axum::extract::rejection::JsonRejection;
use axum::extract::{Path, State};
use axum::http::{HeaderValue, header};
use axum::response::{IntoResponse, Response};
use rustypipe::validate;
use serde::{Deserialize, Serialize};
use utoipa::ToSchema;
use utoipa_axum::router::OpenApiRouter;
use utoipa_axum::routes;

use crate::AppState;
use crate::api::error::{ApiError, ErrorBody};
use crate::auth::CurrentUser;
use crate::sponsorblock::Segment;

pub fn router() -> OpenApiRouter<AppState> {
    OpenApiRouter::new()
        .routes(routes!(segments))
        .routes(routes!(settings, update_settings))
}

#[derive(Debug, Serialize, ToSchema)]
pub struct VideoSegments {
    /// Sorted by start; empty when SponsorBlock is disabled or unreachable.
    pub segments: Vec<Segment>,
}

/// Get SponsorBlock segments
///
/// The server looks them up by hash prefix, so SponsorBlock never learns the video ID.
#[utoipa::path(
    get,
    path = "/videos/{id}/segments",
    tag = "sponsorblock",
    params(("id" = String, Path, description = "YouTube video ID")),
    responses(
        (status = OK, body = VideoSegments),
        (status = BAD_REQUEST, description = "Invalid video ID", body = ErrorBody),
    )
)]
async fn segments(
    State(state): State<AppState>,
    Path(id): Path<String>,
) -> Result<Response, ApiError> {
    validate::video_id(&id).map_err(|_| ApiError::bad_request("invalid video id"))?;
    let segments = state.sponsorblock.segments(&id).await;
    Ok((
        [(
            header::CACHE_CONTROL,
            HeaderValue::from_static("private, max-age=600"),
        )],
        Json(VideoSegments {
            segments: segments.as_ref().clone(),
        }),
    )
        .into_response())
}

/// What the player does in a segment.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, ToSchema)]
#[serde(rename_all = "snake_case")]
pub enum SkipMode {
    /// Skipped without asking.
    Auto,
    /// Marked, with a skip button while inside.
    Button,
    /// Ignored.
    Off,
}

/// Behaviour per skippable category.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, ToSchema)]
#[serde(default)]
pub struct SkipCategories {
    #[schema(required = true)]
    pub sponsor: SkipMode,
    #[schema(required = true)]
    pub selfpromo: SkipMode,
    #[schema(required = true)]
    pub interaction: SkipMode,
    #[schema(required = true)]
    pub intro: SkipMode,
    #[schema(required = true)]
    pub outro: SkipMode,
    #[schema(required = true)]
    pub preview: SkipMode,
    #[schema(required = true)]
    pub hook: SkipMode,
    #[schema(required = true)]
    pub music_offtopic: SkipMode,
    #[schema(required = true)]
    pub filler: SkipMode,
}

impl Default for SkipCategories {
    fn default() -> Self {
        Self {
            sponsor: SkipMode::Auto,
            selfpromo: SkipMode::Button,
            interaction: SkipMode::Button,
            intro: SkipMode::Button,
            outro: SkipMode::Button,
            preview: SkipMode::Button,
            hook: SkipMode::Button,
            music_offtopic: SkipMode::Button,
            filler: SkipMode::Button,
        }
    }
}

/// SponsorBlock preferences. Missing fields take their defaults.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, ToSchema)]
#[serde(default)]
pub struct SponsorBlockSettings {
    #[schema(required = true)]
    pub categories: SkipCategories,
    /// Mark the highlight and offer to jump to it.
    #[schema(required = true)]
    pub highlight: bool,
    /// Label videos that are entirely about a product or service.
    #[schema(required = true)]
    pub full_video_label: bool,
}

impl Default for SponsorBlockSettings {
    fn default() -> Self {
        Self {
            categories: SkipCategories::default(),
            highlight: true,
            full_video_label: true,
        }
    }
}

/// Get SponsorBlock settings
///
/// The signed-in user's SponsorBlock preferences.
#[utoipa::path(
    get,
    path = "/settings/sponsorblock",
    tag = "sponsorblock",
    security(("session" = [])),
    responses(
        (status = OK, body = SponsorBlockSettings),
        (status = UNAUTHORIZED, description = "Not signed in", body = ErrorBody),
    )
)]
async fn settings(
    State(state): State<AppState>,
    CurrentUser(user): CurrentUser,
) -> Result<Json<SponsorBlockSettings>, ApiError> {
    let stored = state.sponsorblock_settings.get(user.id).await?;
    // Stored documents from older versions may not parse; they fall back to the defaults.
    Ok(Json(
        stored
            .and_then(|value| serde_json::from_value(value).ok())
            .unwrap_or_default(),
    ))
}

/// Replace SponsorBlock settings
///
/// Replaces the signed-in user's SponsorBlock preferences.
#[utoipa::path(
    put,
    path = "/settings/sponsorblock",
    tag = "sponsorblock",
    security(("session" = [])),
    request_body = SponsorBlockSettings,
    responses(
        (status = OK, body = SponsorBlockSettings),
        (status = BAD_REQUEST, description = "Invalid body", body = ErrorBody),
        (status = UNAUTHORIZED, description = "Not signed in", body = ErrorBody),
        (status = FORBIDDEN, description = "Foreign origin", body = ErrorBody),
    )
)]
async fn update_settings(
    State(state): State<AppState>,
    CurrentUser(user): CurrentUser,
    body: Result<Json<SponsorBlockSettings>, JsonRejection>,
) -> Result<Json<SponsorBlockSettings>, ApiError> {
    let Json(body) = body?;
    let value = serde_json::to_value(&body).map_err(|_| ApiError::internal())?;
    state.sponsorblock_settings.set(user.id, &value).await?;
    Ok(Json(body))
}
