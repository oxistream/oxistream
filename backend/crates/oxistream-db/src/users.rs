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

#[async_trait::async_trait]
pub trait UserRepository: Send + Sync {
    /// Creates a user. The first user of the instance becomes an admin.
    ///
    /// Fails with [`Error::UsernameTaken`] if the username exists, ignoring case.
    async fn create(&self, user: &NewUser) -> Result<User, Error>;

    /// Looks up a user by username, ignoring case.
    async fn find_by_username(&self, username: &str) -> Result<Option<UserCredentials>, Error>;

    async fn get(&self, id: Uuid) -> Result<Option<User>, Error>;

    /// Returns `false` if the user does not exist.
    async fn set_disabled(&self, id: Uuid, disabled: bool) -> Result<bool, Error>;
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
        "id, username, role, disabled, created_at, updated_at"
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
        created_at: row.try_get("created_at")?,
        updated_at: row.try_get("updated_at")?,
    })
}

#[async_trait::async_trait]
impl UserRepository for PgUserRepository {
    async fn create(&self, user: &NewUser) -> Result<User, Error> {
        let mut tx = self.pool.begin().await?;
        // Serializes sign-ups so exactly one of several concurrent first users becomes admin.
        // Reads are not blocked by this lock mode.
        sqlx::query("LOCK TABLE users IN SHARE ROW EXCLUSIVE MODE")
            .execute(&mut *tx)
            .await?;
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
}
