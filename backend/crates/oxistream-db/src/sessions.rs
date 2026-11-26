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
    /// The provider's session ID (`sid` claim), for back-channel logout.
    pub oidc_sid: Option<String>,
    /// The raw ID token, sent as `id_token_hint` on logout.
    pub oidc_id_token: Option<String>,
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

/// A validated back-channel logout token.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct BackchannelLogout<'a> {
    pub jti: &'a str,
    /// Until when the token could be replayed; its `jti` is kept that long.
    pub expires_at: OffsetDateTime,
    pub issuer: &'a str,
    pub sid: Option<&'a str>,
    pub subject: Option<&'a str>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum LogoutOutcome {
    /// The `jti` was seen before; nothing changed.
    Replayed,
    /// How many sessions were ended.
    Ended(u64),
}

#[async_trait::async_trait]
pub trait SessionRepository: Send + Sync {
    async fn create(&self, session: &NewSession) -> Result<(), Error>;

    /// Returns the session if it exists, has not expired and its user is not disabled.
    async fn find_active(&self, token_hash: &TokenHash) -> Result<Option<ActiveSession>, Error>;

    /// Marks the session as seen now and moves its expiry.
    async fn touch(&self, token_hash: &TokenHash, expires_at: OffsetDateTime) -> Result<(), Error>;

    async fn delete(&self, token_hash: &TokenHash) -> Result<(), Error>;

    /// Deletes the session and returns its stored OIDC ID token, if any.
    async fn delete_returning_id_token(
        &self,
        token_hash: &TokenHash,
    ) -> Result<Option<String>, Error>;

    /// Deletes expired sessions and returns how many were removed.
    async fn delete_expired(&self) -> Result<u64, Error>;

    /// Records the logout token's `jti` and ends the matching sessions in one transaction,
    /// so a failure leaves the token usable for the provider's retry.
    async fn end_sessions_for_logout(
        &self,
        logout: &BackchannelLogout<'_>,
    ) -> Result<LogoutOutcome, Error>;

    /// Forgets logout token IDs that can no longer be replayed.
    async fn delete_expired_logout_jtis(&self) -> Result<u64, Error>;
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
            "INSERT INTO sessions (id, user_id, expires_at, user_agent, ip, oidc_sid, oidc_id_token) \
             VALUES ($1, $2, $3, $4, $5::inet, $6, $7)",
        )
        .bind(session.token_hash.as_slice())
        .bind(session.user_id)
        .bind(session.expires_at)
        .bind(&session.user_agent)
        .bind(session.ip.map(|ip| ip.to_string()))
        .bind(&session.oidc_sid)
        .bind(&session.oidc_id_token)
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

    async fn delete_returning_id_token(
        &self,
        token_hash: &TokenHash,
    ) -> Result<Option<String>, Error> {
        let token: Option<Option<String>> =
            sqlx::query_scalar("DELETE FROM sessions WHERE id = $1 RETURNING oidc_id_token")
                .bind(token_hash.as_slice())
                .fetch_optional(&self.pool)
                .await?;
        Ok(token.flatten())
    }

    async fn delete_expired(&self) -> Result<u64, Error> {
        let result = sqlx::query("DELETE FROM sessions WHERE expires_at <= now()")
            .execute(&self.pool)
            .await?;
        Ok(result.rows_affected())
    }

    async fn end_sessions_for_logout(
        &self,
        logout: &BackchannelLogout<'_>,
    ) -> Result<LogoutOutcome, Error> {
        let mut tx = self.pool.begin().await?;
        let recorded = sqlx::query(
            "INSERT INTO oidc_logout_jti (jti, expires_at) VALUES ($1, $2) \
             ON CONFLICT (jti) DO NOTHING",
        )
        .bind(logout.jti)
        .bind(logout.expires_at)
        .execute(&mut *tx)
        .await?;
        if recorded.rows_affected() == 0 {
            return Ok(LogoutOutcome::Replayed);
        }
        let user_id: Option<Uuid> = match logout.subject {
            Some(subject) => {
                let user_id = sqlx::query_scalar(
                    "SELECT user_id FROM user_identities WHERE issuer = $1 AND subject = $2",
                )
                .bind(logout.issuer)
                .bind(subject)
                .fetch_optional(&mut *tx)
                .await?;
                if user_id.is_none() {
                    tx.commit().await?;
                    return Ok(LogoutOutcome::Ended(0));
                }
                user_id
            }
            None => None,
        };
        let deleted = match (logout.sid, user_id) {
            (Some(sid), user_id) => sqlx::query(
                "DELETE FROM sessions WHERE oidc_sid = $1 AND ($2::uuid IS NULL OR user_id = $2)",
            )
            .bind(sid)
            .bind(user_id)
            .execute(&mut *tx)
            .await?
            .rows_affected(),
            (None, Some(user_id)) => sqlx::query("DELETE FROM sessions WHERE user_id = $1")
                .bind(user_id)
                .execute(&mut *tx)
                .await?
                .rows_affected(),
            (None, None) => 0,
        };
        tx.commit().await?;
        Ok(LogoutOutcome::Ended(deleted))
    }

    async fn delete_expired_logout_jtis(&self) -> Result<u64, Error> {
        let result = sqlx::query("DELETE FROM oidc_logout_jti WHERE expires_at <= now()")
            .execute(&self.pool)
            .await?;
        Ok(result.rows_affected())
    }
}
