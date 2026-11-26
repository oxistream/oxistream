use axum::Json;
use axum::http::StatusCode;
use axum::response::{IntoResponse, Response};
use serde::Serialize;
use utoipa::ToSchema;

use crate::youtube::YouTubeError;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, ToSchema)]
#[serde(rename_all = "snake_case")]
pub enum ErrorCode {
    BadRequest,
    NotFound,
    /// The content exists but cannot be played or shown (private, age-restricted, ...).
    Unavailable,
    /// YouTube is rate limiting or blocking this instance.
    RateLimited,
    /// Proof-of-origin token generation via `rustypipe-botguard` failed.
    BotguardFailed,
    UpstreamError,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, ToSchema)]
pub struct ErrorBody {
    pub code: ErrorCode,
    pub message: String,
    /// Machine-readable detail, e.g. the unavailability reason.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub reason: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ApiError {
    pub status: StatusCode,
    pub body: ErrorBody,
}

impl ApiError {
    pub fn new(status: StatusCode, code: ErrorCode, message: impl Into<String>) -> Self {
        Self {
            status,
            body: ErrorBody {
                code,
                message: message.into(),
                reason: None,
            },
        }
    }

    pub fn bad_request(message: impl Into<String>) -> Self {
        Self::new(StatusCode::BAD_REQUEST, ErrorCode::BadRequest, message)
    }
}

impl IntoResponse for ApiError {
    fn into_response(self) -> Response {
        (self.status, Json(self.body)).into_response()
    }
}

impl From<YouTubeError> for ApiError {
    fn from(err: YouTubeError) -> Self {
        match err {
            YouTubeError::InvalidInput(message) => Self::bad_request(message),
            YouTubeError::NotFound(_) => {
                Self::new(StatusCode::NOT_FOUND, ErrorCode::NotFound, "not found")
            }
            YouTubeError::Unavailable { reason, message } => {
                let mut err = Self::new(StatusCode::NOT_FOUND, ErrorCode::Unavailable, message);
                err.body.reason = Some(reason);
                err
            }
            YouTubeError::RateLimited(detail) => {
                tracing::warn!(%detail, "YouTube is blocking requests");
                Self::new(
                    StatusCode::SERVICE_UNAVAILABLE,
                    ErrorCode::RateLimited,
                    "YouTube is rate limiting this instance, try again later",
                )
            }
            YouTubeError::Botguard(detail) => {
                tracing::warn!(%detail, "botguard failed");
                Self::new(
                    StatusCode::SERVICE_UNAVAILABLE,
                    ErrorCode::BotguardFailed,
                    "could not generate a proof-of-origin token",
                )
            }
            YouTubeError::Upstream(detail) => {
                tracing::warn!(%detail, "YouTube request failed");
                Self::new(
                    StatusCode::BAD_GATEWAY,
                    ErrorCode::UpstreamError,
                    "YouTube request failed",
                )
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn maps_youtube_errors_to_statuses() {
        let cases = [
            (
                YouTubeError::InvalidInput("x".into()),
                StatusCode::BAD_REQUEST,
                ErrorCode::BadRequest,
            ),
            (
                YouTubeError::NotFound("x".into()),
                StatusCode::NOT_FOUND,
                ErrorCode::NotFound,
            ),
            (
                YouTubeError::Unavailable {
                    reason: "private".into(),
                    message: "x".into(),
                },
                StatusCode::NOT_FOUND,
                ErrorCode::Unavailable,
            ),
            (
                YouTubeError::RateLimited("x".into()),
                StatusCode::SERVICE_UNAVAILABLE,
                ErrorCode::RateLimited,
            ),
            (
                YouTubeError::Botguard("x".into()),
                StatusCode::SERVICE_UNAVAILABLE,
                ErrorCode::BotguardFailed,
            ),
            (
                YouTubeError::Upstream("x".into()),
                StatusCode::BAD_GATEWAY,
                ErrorCode::UpstreamError,
            ),
        ];
        for (err, status, code) in cases {
            let api = ApiError::from(err);
            assert_eq!((api.status, api.body.code), (status, code));
        }
    }

    #[test]
    fn unavailable_carries_reason() {
        let api = ApiError::from(YouTubeError::Unavailable {
            reason: "age-restricted".into(),
            message: "Sign in to confirm your age".into(),
        });
        let json = serde_json::to_value(&api.body).unwrap();
        assert_eq!(json["code"], "unavailable");
        assert_eq!(json["reason"], "age-restricted");
    }
}
