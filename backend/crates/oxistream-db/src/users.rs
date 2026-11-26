use std::fmt;
use std::str::FromStr;

use sqlx::postgres::PgRow;
use sqlx::{PgPool, Row};
use time::OffsetDateTime;
use uuid::Uuid;

use crate::Error;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Role {
    User,
    Admin,
}

impl Role {
    pub fn as_str(self) -> &'static str {
        match self {
            Self::User => "user",
            Self::Admin => "admin",
        }
    }
}

impl fmt::Display for Role {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(self.as_str())
    }
}

impl FromStr for Role {
    type Err = String;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s {
            "user" => Ok(Self::User),
            "admin" => Ok(Self::Admin),
            _ => Err(format!("unknown role {s:?}")),
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct User {
    pub id: Uuid,
    pub username: String,
    pub role: Role,
    pub disabled: bool,
    /// Local accounts have a password; single sign-on accounts do not.
    pub has_password: bool,
    pub created_at: OffsetDateTime,
    pub updated_at: OffsetDateTime,
}

/// A user together with the stored password hash, for verifying a login.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct UserCredentials {
    pub user: User,
    /// `None` for accounts without a local password.
    pub password_hash: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct NewUser {
    pub username: String,
    pub password_hash: Option<String>,
}

/// An account at the `OpenID` Connect provider.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Identity {
    pub issuer: String,
    pub subject: String,
}

/// Result of [`UserRepository::delete`].
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum DeleteOutcome {
    Deleted,
    NotFound,
    /// The user is the last admin while other users remain.
    LastAdmin,
}

/// Username candidates for a new account: attempt `0` is the preferred name, later attempts
/// add a suffix.
pub type UsernameCandidates<'a> = dyn Fn(u32) -> String + Send + Sync + 'a;

#[async_trait::async_trait]
pub trait UserRepository: Send + Sync {
    /// Creates a user. The first user of the instance becomes an admin.
    ///
    /// Fails with [`Error::UsernameTaken`] if the username exists, ignoring case.
    async fn create(&self, user: &NewUser) -> Result<User, Error>;

    async fn find_by_identity(&self, identity: &Identity) -> Result<Option<User>, Error>;

    /// Creates a user without password linked to `identity`, using the first free username
    /// from `usernames`. The user becomes an admin if `admin` is set or it is the first user of
    /// the instance. Returns the already linked user if another request provisioned it first.
    async fn provision(
        &self,
        identity: &Identity,
        usernames: &UsernameCandidates<'_>,
        admin: bool,
    ) -> Result<User, Error>;

    /// Sets the role, except that the last admin is never demoted. Returns the updated user,
    /// or `None` if it does not exist.
    async fn sync_role(&self, id: Uuid, role: Role) -> Result<Option<User>, Error>;

    /// Looks up a user by username, ignoring case.
    async fn find_by_username(&self, username: &str) -> Result<Option<UserCredentials>, Error>;

    async fn get(&self, id: Uuid) -> Result<Option<User>, Error>;

    /// Returns `false` if the user does not exist.
    async fn set_disabled(&self, id: Uuid, disabled: bool) -> Result<bool, Error>;

    /// The stored password hash; `None` if the user does not exist or has no password.
    async fn password_hash(&self, id: Uuid) -> Result<Option<String>, Error>;

    /// Replaces the password of a local account. Returns `false` if the user does not exist or
    /// has no password, since single sign-on accounts never get one.
    async fn set_password(&self, id: Uuid, password_hash: &str) -> Result<bool, Error>;

    /// Deletes the user and, by cascade, everything they own, including their sessions. The
    /// last admin is kept unless no other users remain.
    async fn delete(&self, id: Uuid) -> Result<DeleteOutcome, Error>;
}

#[derive(Debug, Clone)]
pub struct PgUserRepository {
    pool: PgPool,
}

impl PgUserRepository {
    pub fn new(pool: PgPool) -> Self {
        Self { pool }
    }
}

macro_rules! user_columns {
    () => {
        "id, username, role, disabled, password_hash IS NOT NULL AS has_password, \
         created_at, updated_at"
    };
}

pub(crate) fn user_from_row(row: &PgRow) -> Result<User, sqlx::Error> {
    let role: String = row.try_get("role")?;
    Ok(User {
        id: row.try_get("id")?,
        username: row.try_get("username")?,
        role: role
            .parse()
            .map_err(|err: String| sqlx::Error::ColumnDecode {
                index: "role".into(),
                source: err.into(),
            })?,
        disabled: row.try_get("disabled")?,
        has_password: row.try_get("has_password")?,
        created_at: row.try_get("created_at")?,
        updated_at: row.try_get("updated_at")?,
    })
}

/// Serializes sign-ups and role changes so exactly one of several concurrent first users
/// becomes admin and the last admin cannot be demoted concurrently. Reads are not blocked by
/// this lock mode.
async fn lock_users(tx: &mut sqlx::PgConnection) -> Result<(), sqlx::Error> {
    sqlx::query("LOCK TABLE users IN SHARE ROW EXCLUSIVE MODE")
        .execute(tx)
        .await?;
    Ok(())
}

const MAX_USERNAME_ATTEMPTS: u32 = 1000;

async fn user_by_identity<'e>(
    executor: impl sqlx::PgExecutor<'e>,
    identity: &Identity,
) -> Result<Option<User>, sqlx::Error> {
    let row = sqlx::query(concat!(
        "SELECT ",
        user_columns!(),
        " FROM users WHERE id = \
         (SELECT user_id FROM user_identities WHERE issuer = $1 AND subject = $2)"
    ))
    .bind(&identity.issuer)
    .bind(&identity.subject)
    .fetch_optional(executor)
    .await?;
    row.as_ref().map(user_from_row).transpose()
}

#[async_trait::async_trait]
impl UserRepository for PgUserRepository {
    async fn create(&self, user: &NewUser) -> Result<User, Error> {
        let mut tx = self.pool.begin().await?;
        lock_users(&mut tx).await?;
        let row = sqlx::query(concat!(
            "INSERT INTO users (username, password_hash, role) \
             VALUES ($1, $2, CASE WHEN EXISTS (SELECT 1 FROM users) THEN 'user' ELSE 'admin' END) \
             RETURNING ",
            user_columns!()
        ))
        .bind(&user.username)
        .bind(&user.password_hash)
        .fetch_one(&mut *tx)
        .await
        .map_err(|err| match &err {
            sqlx::Error::Database(db) if db.constraint() == Some("users_username_lower_key") => {
                Error::UsernameTaken
            }
            _ => Error::Sqlx(err),
        })?;
        let user = user_from_row(&row)?;
        tx.commit().await?;
        Ok(user)
    }

    async fn find_by_identity(&self, identity: &Identity) -> Result<Option<User>, Error> {
        Ok(user_by_identity(&self.pool, identity).await?)
    }

    async fn provision(
        &self,
        identity: &Identity,
        usernames: &UsernameCandidates<'_>,
        admin: bool,
    ) -> Result<User, Error> {
        let mut tx = self.pool.begin().await?;
        lock_users(&mut tx).await?;
        // Provisioning holds the lock, so a concurrent callback for the same identity has
        // either committed by now or waits for this transaction.
        if let Some(user) = user_by_identity(&mut *tx, identity).await? {
            return Ok(user);
        }
        let mut username = None;
        for attempt in 0..MAX_USERNAME_ATTEMPTS {
            let candidate = usernames(attempt);
            let taken: bool = sqlx::query_scalar(
                "SELECT EXISTS (SELECT 1 FROM users WHERE lower(username) = lower($1))",
            )
            .bind(&candidate)
            .fetch_one(&mut *tx)
            .await?;
            if !taken {
                username = Some(candidate);
                break;
            }
        }
        let username = username.ok_or(Error::UsernameTaken)?;
        let row = sqlx::query(concat!(
            "INSERT INTO users (username, role) VALUES ($1, \
             CASE WHEN $2 OR NOT EXISTS (SELECT 1 FROM users) THEN 'admin' ELSE 'user' END) \
             RETURNING ",
            user_columns!()
        ))
        .bind(&username)
        .bind(admin)
        .fetch_one(&mut *tx)
        .await?;
        let user = user_from_row(&row)?;
        sqlx::query("INSERT INTO user_identities (issuer, subject, user_id) VALUES ($1, $2, $3)")
            .bind(&identity.issuer)
            .bind(&identity.subject)
            .bind(user.id)
            .execute(&mut *tx)
            .await?;
        tx.commit().await?;
        Ok(user)
    }

    async fn sync_role(&self, id: Uuid, role: Role) -> Result<Option<User>, Error> {
        let mut tx = self.pool.begin().await?;
        lock_users(&mut tx).await?;
        let row = sqlx::query(concat!(
            "UPDATE users SET role = $2, updated_at = now() \
             WHERE id = $1 AND role <> $2 AND ($2 = 'admin' OR EXISTS \
                (SELECT 1 FROM users WHERE role = 'admin' AND id <> $1)) \
             RETURNING ",
            user_columns!()
        ))
        .bind(id)
        .bind(role.as_str())
        .fetch_optional(&mut *tx)
        .await?;
        let user = match row {
            Some(row) => Some(user_from_row(&row)?),
            None => sqlx::query(concat!(
                "SELECT ",
                user_columns!(),
                " FROM users WHERE id = $1"
            ))
            .bind(id)
            .fetch_optional(&mut *tx)
            .await?
            .as_ref()
            .map(user_from_row)
            .transpose()?,
        };
        tx.commit().await?;
        Ok(user)
    }

    async fn find_by_username(&self, username: &str) -> Result<Option<UserCredentials>, Error> {
        let row = sqlx::query(concat!(
            "SELECT ",
            user_columns!(),
            ", password_hash FROM users WHERE lower(username) = lower($1)"
        ))
        .bind(username)
        .fetch_optional(&self.pool)
        .await?;
        row.map(|row| {
            Ok(UserCredentials {
                user: user_from_row(&row)?,
                password_hash: row.try_get("password_hash")?,
            })
        })
        .transpose()
    }

    async fn get(&self, id: Uuid) -> Result<Option<User>, Error> {
        let row = sqlx::query(concat!(
            "SELECT ",
            user_columns!(),
            " FROM users WHERE id = $1"
        ))
        .bind(id)
        .fetch_optional(&self.pool)
        .await?;
        Ok(row.as_ref().map(user_from_row).transpose()?)
    }

    async fn set_disabled(&self, id: Uuid, disabled: bool) -> Result<bool, Error> {
        let result =
            sqlx::query("UPDATE users SET disabled = $2, updated_at = now() WHERE id = $1")
                .bind(id)
                .bind(disabled)
                .execute(&self.pool)
                .await?;
        Ok(result.rows_affected() > 0)
    }

    async fn password_hash(&self, id: Uuid) -> Result<Option<String>, Error> {
        let hash: Option<Option<String>> =
            sqlx::query_scalar("SELECT password_hash FROM users WHERE id = $1")
                .bind(id)
                .fetch_optional(&self.pool)
                .await?;
        Ok(hash.flatten())
    }

    async fn set_password(&self, id: Uuid, password_hash: &str) -> Result<bool, Error> {
        let result = sqlx::query(
            "UPDATE users SET password_hash = $2, updated_at = now() \
             WHERE id = $1 AND password_hash IS NOT NULL",
        )
        .bind(id)
        .bind(password_hash)
        .execute(&self.pool)
        .await?;
        Ok(result.rows_affected() > 0)
    }

    async fn delete(&self, id: Uuid) -> Result<DeleteOutcome, Error> {
        let mut tx = self.pool.begin().await?;
        lock_users(&mut tx).await?;
        let role: Option<String> = sqlx::query_scalar("SELECT role FROM users WHERE id = $1")
            .bind(id)
            .fetch_optional(&mut *tx)
            .await?;
        let Some(role) = role else {
            return Ok(DeleteOutcome::NotFound);
        };
        if role == Role::Admin.as_str() {
            let blocked: bool = sqlx::query_scalar(
                "SELECT NOT EXISTS (SELECT 1 FROM users WHERE role = 'admin' AND id <> $1) \
                    AND EXISTS (SELECT 1 FROM users WHERE id <> $1)",
            )
            .bind(id)
            .fetch_one(&mut *tx)
            .await?;
            if blocked {
                return Ok(DeleteOutcome::LastAdmin);
            }
        }
        sqlx::query("DELETE FROM users WHERE id = $1")
            .bind(id)
            .execute(&mut *tx)
            .await?;
        tx.commit().await?;
        Ok(DeleteOutcome::Deleted)
    }
}
