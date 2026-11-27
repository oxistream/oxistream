//! How long the watch history is kept, and the cleanup that enforces it.

use std::str::FromStr;
use std::sync::Arc;
use std::time::Duration;

use oxistream_db::{AdvisoryLock, HistoryRepository, LockKey, PgPool};
use serde::{Deserialize, Serialize};
use utoipa::ToSchema;

const CLEANUP_INTERVAL: Duration = Duration::from_hours(1);

/// How long watch history entries are kept after they were last watched.
#[derive(
    Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Default, Serialize, Deserialize, ToSchema,
)]
pub enum Retention {
    #[serde(rename = "1d")]
    Day,
    #[serde(rename = "7d")]
    Week,
    #[serde(rename = "14d")]
    TwoWeeks,
    #[serde(rename = "30d")]
    Month,
    #[serde(rename = "90d")]
    ThreeMonths,
    #[serde(rename = "180d")]
    SixMonths,
    #[serde(rename = "365d")]
    Year,
    #[default]
    #[serde(rename = "forever")]
    Forever,
}

impl Retention {
    pub const ALL: [Self; 8] = [
        Self::Day,
        Self::Week,
        Self::TwoWeeks,
        Self::Month,
        Self::ThreeMonths,
        Self::SixMonths,
        Self::Year,
        Self::Forever,
    ];

    /// `None` for forever.
    pub fn days(self) -> Option<i32> {
        match self {
            Self::Day => Some(1),
            Self::Week => Some(7),
            Self::TwoWeeks => Some(14),
            Self::Month => Some(30),
            Self::ThreeMonths => Some(90),
            Self::SixMonths => Some(180),
            Self::Year => Some(365),
            Self::Forever => None,
        }
    }

    /// Unknown day counts keep the history forever.
    pub fn from_days(days: Option<i32>) -> Self {
        Self::ALL
            .into_iter()
            .find(|r| r.days() == days)
            .unwrap_or_default()
    }

    pub fn as_str(self) -> &'static str {
        match self {
            Self::Day => "1d",
            Self::Week => "7d",
            Self::TwoWeeks => "14d",
            Self::Month => "30d",
            Self::ThreeMonths => "90d",
            Self::SixMonths => "180d",
            Self::Year => "365d",
            Self::Forever => "forever",
        }
    }
}

impl FromStr for Retention {
    type Err = String;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        let s = s.to_ascii_lowercase();
        Self::ALL
            .into_iter()
            .find(|r| r.as_str() == s)
            .ok_or_else(|| {
                "expected one of `forever`, `365d`, `180d`, `90d`, `30d`, `14d`, `7d`, `1d`".into()
            })
    }
}

/// Deletes expired history entries every hour. The advisory lock keeps replicas from doing
/// the same work at once; the deletion itself is idempotent.
pub fn spawn_cleanup(pool: PgPool, history: Arc<dyn HistoryRepository>) {
    tokio::spawn(async move {
        let mut interval = tokio::time::interval(CLEANUP_INTERVAL);
        loop {
            interval.tick().await;
            if let Err(err) = cleanup(&pool, history.as_ref()).await {
                tracing::warn!(error = ?err, "could not delete expired watch history");
            }
        }
    });
}

/// Runs one cleanup unless another replica is at it. Returns the number of deleted entries.
pub async fn cleanup(pool: &PgPool, history: &dyn HistoryRepository) -> anyhow::Result<u64> {
    let Some(lock) = AdvisoryLock::try_acquire(pool, LockKey::HistoryRetention).await? else {
        return Ok(0);
    };
    let deleted = history.delete_expired().await;
    lock.release().await?;
    let deleted = deleted?;
    if deleted > 0 {
        tracing::debug!(count = deleted, "deleted expired watch history");
    }
    Ok(deleted)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn round_trips_days_and_names() {
        for r in Retention::ALL {
            assert_eq!(Retention::from_days(r.days()), r);
            assert_eq!(r.as_str().parse::<Retention>(), Ok(r));
            assert_eq!(
                serde_json::to_string(&r).unwrap(),
                format!("\"{}\"", r.as_str())
            );
        }
        assert_eq!(Retention::from_days(Some(2)), Retention::Forever);
        assert!("2d".parse::<Retention>().is_err());
    }

    #[test]
    fn orders_by_length() {
        assert!(Retention::Day < Retention::Year);
        assert!(Retention::Year < Retention::Forever);
        assert_eq!(Retention::Forever.min(Retention::Month), Retention::Month);
    }
}
