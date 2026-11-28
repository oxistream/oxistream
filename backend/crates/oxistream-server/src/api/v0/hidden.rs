use std::collections::HashSet;

use axum::extract::{FromRequestParts, Path, State};
use axum::http::StatusCode;
use axum::http::request::Parts;
use rustypipe::validate;
use utoipa_axum::router::OpenApiRouter;
use utoipa_axum::routes;

use crate::AppState;
use crate::api::dto::VideoSummary;
use crate::api::error::{ApiError, ErrorBody};
use crate::auth::{CurrentUser, OptionalUser};

pub fn router() -> OpenApiRouter<AppState> {
    OpenApiRouter::new().routes(routes!(hide, unhide))
}

/// Videos the requesting user is not interested in; empty for anonymous requests.
#[derive(Debug, Default)]
pub struct HiddenVideos(HashSet<String>);

impl HiddenVideos {
    pub fn retain(&self, items: &mut Vec<VideoSummary>) {
        if self.0.is_empty() {
            return;
        }
        items.retain(|video| !self.0.contains(&video.id));
    }
}

impl FromRequestParts<AppState> for HiddenVideos {
    type Rejection = ApiError;

    async fn from_request_parts(parts: &mut Parts, state: &AppState) -> Result<Self, ApiError> {
        let Ok(OptionalUser(Some(user))) = OptionalUser::from_request_parts(parts, state).await
        else {
            return Ok(Self::default());
        };
        Ok(Self(state.hidden.hidden_ids(user.id).await?))
    }
}

fn check_video_id(id: &str) -> Result<(), ApiError> {
    validate::video_id(id).map_err(|_| ApiError::bad_request("invalid video id"))
}

/// Mark a video as not interesting. It is left out of the subscription feed, "Continue
/// watching" and trending. Idempotent.
#[utoipa::path(
    put,
    path = "/hidden-videos/{video_id}",
    tag = "hidden-videos",
    security(("session" = [])),
    params(("video_id" = String, Path, description = "YouTube video ID")),
    responses(
        (status = NO_CONTENT, description = "Hidden"),
        (status = BAD_REQUEST, description = "Invalid video ID", body = ErrorBody),
        (status = UNAUTHORIZED, description = "Not signed in", body = ErrorBody),
        (status = FORBIDDEN, description = "Foreign origin", body = ErrorBody),
    )
)]
async fn hide(
    State(state): State<AppState>,
    CurrentUser(user): CurrentUser,
    Path(video_id): Path<String>,
) -> Result<StatusCode, ApiError> {
    check_video_id(&video_id)?;
    state.hidden.hide(user.id, &video_id).await?;
    Ok(StatusCode::NO_CONTENT)
}

/// Show a hidden video again. Succeeds if it is not hidden, too.
#[utoipa::path(
    delete,
    path = "/hidden-videos/{video_id}",
    tag = "hidden-videos",
    security(("session" = [])),
    params(("video_id" = String, Path, description = "YouTube video ID")),
    responses(
        (status = NO_CONTENT, description = "Not hidden anymore"),
        (status = BAD_REQUEST, description = "Invalid video ID", body = ErrorBody),
        (status = UNAUTHORIZED, description = "Not signed in", body = ErrorBody),
        (status = FORBIDDEN, description = "Foreign origin", body = ErrorBody),
    )
)]
async fn unhide(
    State(state): State<AppState>,
    CurrentUser(user): CurrentUser,
    Path(video_id): Path<String>,
) -> Result<StatusCode, ApiError> {
    check_video_id(&video_id)?;
    state.hidden.unhide(user.id, &video_id).await?;
    Ok(StatusCode::NO_CONTENT)
}
