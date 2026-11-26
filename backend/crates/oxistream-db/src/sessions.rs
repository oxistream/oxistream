use std::net::IpAddr;

use sqlx::{PgPool, Row};
use time::OffsetDateTime;
use uuid::Uuid;

use crate::Error;
use crate::users::{User, user_from_row};

/// SHA-256 of a session token.
pub type TokenHash = [u8; 32];

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct NewSession {
    pub token_hash: TokenHash,
    pub user_id: Uuid,
    pub expires_at: OffsetDateTime,
    pub user_agent: Option<String>,
    pub ip: Option<IpAddr>,
    pub oidc_sid: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Session {
    pub user_id: Uuid,
    pub created_at: OffsetDateTime,
    pub last_seen_at: OffsetDateTime,
    pub expires_at: OffsetDateTime,
}

/// A valid session and its user.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ActiveSession {
    pub session: Session,
    pub user: User,
}

#[async_trait::async_trait]
pub trait SessionRepository: Send + Sync {
    async fn create(&self, session: &NewSession) -> Result<(), Error>;

    /// Returns the session if it exists, has not expired and its user is not disabled.
    async fn find_active(&self, token_hash: &TokenHash) -> Result<Option<ActiveSession>, Error>;

    /// Marks the session as seen now and moves its expiry.
    async fn touch(&self, token_hash: &TokenHash, expires_at: OffsetDateTime) -> Result<(), Error>;

    async fn delete(&self, token_hash: &TokenHash) -> Result<(), Error>;

    /// Deletes expired sessions and returns how many were removed.
    async fn delete_expired(&self) -> Result<u64, Error>;
}

#[derive(Debug, Clone)]
pub struct PgSessionRepository {
    pool: PgPool,
}

impl PgSessionRepository {
    pub fn new(pool: PgPool) -> Self {
        Self { pool }
    }
}

#[async_trait::async_trait]
impl SessionRepository for PgSessionRepository {
    async fn create(&self, session: &NewSession) -> Result<(), Error> {
        sqlx::query(
            "INSERT INTO sessions (id, user_id, expires_at, user_agent, ip, oidc_sid) \
             VALUES ($1, $2, $3, $4, $5::inet, $6)",
        )
        .bind(session.token_hash.as_slice())
        .bind(session.user_id)
        .bind(session.expires_at)
        .bind(&session.user_agent)
        .bind(session.ip.map(|ip| ip.to_string()))
        .bind(&session.oidc_sid)
        .execute(&self.pool)
        .await?;
        Ok(())
    }

    async fn find_active(&self, token_hash: &TokenHash) -> Result<Option<ActiveSession>, Error> {
        let row = sqlx::query(
            "SELECT s.created_at AS s_created_at, s.last_seen_at, s.expires_at, \
                u.id, u.username, u.role, u.disabled, u.created_at, u.updated_at \
             FROM sessions s JOIN users u ON u.id = s.user_id \
             WHERE s.id = $1 AND s.expires_at > now() AND NOT u.disabled",
        )
        .bind(token_hash.as_slice())
        .fetch_optional(&self.pool)
        .await?;
        row.map(|row| {
            let user = user_from_row(&row)?;
            Ok(ActiveSession {
                session: Session {
                    user_id: user.id,
                    created_at: row.try_get("s_created_at")?,
                    last_seen_at: row.try_get("last_seen_at")?,
                    expires_at: row.try_get("expires_at")?,
                },
                user,
            })
        })
        .transpose()
    }

    async fn touch(&self, token_hash: &TokenHash, expires_at: OffsetDateTime) -> Result<(), Error> {
        sqlx::query("UPDATE sessions SET last_seen_at = now(), expires_at = $2 WHERE id = $1")
            .bind(token_hash.as_slice())
            .bind(expires_at)
            .execute(&self.pool)
            .await?;
        Ok(())
    }

    async fn delete(&self, token_hash: &TokenHash) -> Result<(), Error> {
        sqlx::query("DELETE FROM sessions WHERE id = $1")
            .bind(token_hash.as_slice())
            .execute(&self.pool)
            .await?;
        Ok(())
    }

    async fn delete_expired(&self) -> Result<u64, Error> {
        let result = sqlx::query("DELETE FROM sessions WHERE expires_at <= now()")
            .execute(&self.pool)
            .await?;
        Ok(result.rows_affected())
    }
}
