use std::collections::BTreeMap;

use axum::Json;
use axum::extract::rejection::JsonRejection;
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
    /// No valid session.
    Unauthorized,
    /// The signed-in user lacks the required role.
    Forbidden,
    /// A state-changing request did not come from this instance's origin.
    OriginRejected,
    /// Unknown username or wrong password.
    InvalidCredentials,
    AccountDisabled,
    /// Some fields are invalid; see `fields`.
    ValidationFailed,
    LocalAuthDisabled,
    RegistrationDisabled,
    TooManyRequests,
    /// A per-user limit, e.g. on playlist size, is reached.
    LimitReached,
    /// The account has no password, e.g. because it signs in via single sign-on.
    NoPassword,
    /// The change would leave the instance without an (active) admin.
    LastAdmin,
    /// Admins cannot change, disable or delete their own account via the admin API.
    OwnAccount,
    Internal,
}

/// Why a request field was rejected.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, ToSchema)]
#[serde(rename_all = "snake_case")]
pub enum FieldError {
    Required,
    /// Wrong characters or length, e.g. for usernames.
    Format,
    /// Shorter than the minimum length.
    Length,
    /// Longer than the maximum length.
    TooLong,
    /// Already in use.
    Taken,
    /// Does not match, e.g. a wrong current password or a mistyped confirmation.
    Mismatch,
    /// Not one of the allowed values, or a duplicate.
    Invalid,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, ToSchema)]
pub struct ErrorBody {
    pub code: ErrorCode,
    pub message: String,
    /// Machine-readable detail, e.g. the unavailability reason.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub reason: Option<String>,
    /// Per-field errors for `validation_failed`.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub fields: Option<BTreeMap<String, FieldError>>,
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
                fields: None,
            },
        }
    }

    pub fn bad_request(message: impl Into<String>) -> Self {
        Self::new(StatusCode::BAD_REQUEST, ErrorCode::BadRequest, message)
    }

    pub fn unauthorized() -> Self {
        Self::new(
            StatusCode::UNAUTHORIZED,
            ErrorCode::Unauthorized,
            "sign in required",
        )
    }

    pub fn forbidden() -> Self {
        Self::new(
            StatusCode::FORBIDDEN,
            ErrorCode::Forbidden,
            "admin role required",
        )
    }

    pub fn origin_rejected() -> Self {
        Self::new(
            StatusCode::FORBIDDEN,
            ErrorCode::OriginRejected,
            "request origin is not allowed",
        )
    }

    pub fn too_many_requests() -> Self {
        Self::new(
            StatusCode::TOO_MANY_REQUESTS,
            ErrorCode::TooManyRequests,
            "too many attempts, try again later",
        )
    }

    pub fn internal() -> Self {
        Self::new(
            StatusCode::INTERNAL_SERVER_ERROR,
            ErrorCode::Internal,
            "internal server error",
        )
    }

    pub fn validation(status: StatusCode, fields: BTreeMap<String, FieldError>) -> Self {
        let mut err = Self::new(status, ErrorCode::ValidationFailed, "invalid input");
        err.body.fields = Some(fields);
        err
    }
}

impl From<oxistream_db::Error> for ApiError {
    fn from(err: oxistream_db::Error) -> Self {
        tracing::error!(error = ?err, "database error");
        Self::internal()
    }
}

impl From<anyhow::Error> for ApiError {
    fn from(err: anyhow::Error) -> Self {
        tracing::error!(error = ?err, "internal error");
        Self::internal()
    }
}

impl From<JsonRejection> for ApiError {
    fn from(err: JsonRejection) -> Self {
        Self::new(err.status(), ErrorCode::BadRequest, err.body_text())
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
