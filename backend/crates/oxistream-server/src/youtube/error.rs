use rustypipe::error::{Error as RpError, ExtractionError, UnavailabilityReason};

/// A classified upstream failure. Cheap to clone so cached lookups can share it.
#[derive(Debug, Clone, PartialEq, Eq, thiserror::Error)]
pub enum YouTubeError {
    #[error("invalid input: {0}")]
    InvalidInput(String),
    #[error("not found: {0}")]
    NotFound(String),
    #[error("content unavailable ({reason}): {message}")]
    Unavailable { reason: String, message: String },
    #[error("blocked by YouTube: {0}")]
    RateLimited(String),
    #[error("PO token generation failed: {0}")]
    Botguard(String),
    #[error("upstream error: {0}")]
    Upstream(String),
}

impl From<RpError> for YouTubeError {
    fn from(err: RpError) -> Self {
        let message = err.to_string();
        match err {
            RpError::Extraction(ExtractionError::NotFound { .. }) | RpError::HttpStatus(404, _) => {
                Self::NotFound(message)
            }
            RpError::Extraction(ExtractionError::Unavailable { reason, msg }) => match reason {
                UnavailabilityReason::IpBan
                | UnavailabilityReason::VpnBan
                | UnavailabilityReason::Captcha
                | UnavailabilityReason::TryAgain => Self::RateLimited(message),
                reason => Self::Unavailable {
                    reason: reason.to_string(),
                    message: msg,
                },
            },
            RpError::HttpStatus(429, _) => Self::RateLimited(message),
            RpError::Extraction(ExtractionError::Botguard(_)) => Self::Botguard(message),
            RpError::Extraction(ExtractionError::BadRequest(_)) => Self::InvalidInput(message),
            _ => Self::Upstream(message),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn unavailable(reason: UnavailabilityReason) -> RpError {
        RpError::Extraction(ExtractionError::Unavailable {
            reason,
            msg: "nope".into(),
        })
    }

    #[test]
    fn classifies_not_found() {
        let err = RpError::Extraction(ExtractionError::NotFound {
            id: "x".into(),
            msg: "gone".into(),
        });
        assert!(matches!(YouTubeError::from(err), YouTubeError::NotFound(_)));
        assert!(matches!(
            YouTubeError::from(RpError::HttpStatus(404, "".into())),
            YouTubeError::NotFound(_)
        ));
    }

    #[test]
    fn classifies_unavailable() {
        assert_eq!(
            YouTubeError::from(unavailable(UnavailabilityReason::Private)),
            YouTubeError::Unavailable {
                reason: "private".into(),
                message: "nope".into()
            }
        );
    }

    #[test]
    fn classifies_bot_detection_as_rate_limited() {
        for reason in [
            UnavailabilityReason::IpBan,
            UnavailabilityReason::Captcha,
            UnavailabilityReason::TryAgain,
        ] {
            assert!(matches!(
                YouTubeError::from(unavailable(reason)),
                YouTubeError::RateLimited(_)
            ));
        }
        assert!(matches!(
            YouTubeError::from(RpError::HttpStatus(429, "slow down".into())),
            YouTubeError::RateLimited(_)
        ));
        assert!(matches!(
            YouTubeError::from(RpError::Extraction(ExtractionError::Botguard("x".into()))),
            YouTubeError::Botguard(_)
        ));
    }

    #[test]
    fn classifies_other_errors_as_upstream() {
        assert!(matches!(
            YouTubeError::from(RpError::Http("timeout".into())),
            YouTubeError::Upstream(_)
        ));
        assert!(matches!(
            YouTubeError::from(RpError::Extraction(ExtractionError::InvalidData(
                "bad".into()
            ))),
            YouTubeError::Upstream(_)
        ));
    }
}
