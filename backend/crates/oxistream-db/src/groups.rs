use sqlx::PgPool;
use uuid::Uuid;

use crate::Error;

const UNIQUE_NAME: &str = "subscription_groups_user_name_idx";

/// A user's group of subscribed channels.
#[derive(Debug, Clone, PartialEq, Eq, sqlx::FromRow)]
pub struct SubscriptionGroup {
    pub id: Uuid,
    pub name: String,
    pub show_on_home: bool,
    /// Subscribed channels in the group, by ID.
    pub channel_ids: Vec<String>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct NewSubscriptionGroup {
    pub name: String,
    pub show_on_home: bool,
    /// Channels the user is not subscribed to are left out.
    pub channel_ids: Vec<String>,
}

/// Changed fields; `None` keeps the stored value.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct SubscriptionGroupUpdate {
    pub name: Option<String>,
    pub show_on_home: Option<bool>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum GroupChannelOutcome {
    Done,
    GroupNotFound,
    NotSubscribed,
}

/// Mutations take the owner and treat other users' groups as nonexistent. Names are unique per
/// user, ignoring case; a clash fails with [`Error::NameTaken`].
#[async_trait::async_trait]
pub trait SubscriptionGroupRepository: Send + Sync {
    /// A user's groups, in their order.
    async fn list(&self, user_id: Uuid) -> Result<Vec<SubscriptionGroup>, Error>;

    async fn get(&self, user_id: Uuid, id: Uuid) -> Result<Option<SubscriptionGroup>, Error>;

    /// Appends a group. Fails with [`Error::LimitReached`] if the user already has `max_groups`.
    async fn create(
        &self,
        user_id: Uuid,
        group: &NewSubscriptionGroup,
        max_groups: u32,
    ) -> Result<SubscriptionGroup, Error>;

    async fn update(
        &self,
        user_id: Uuid,
        id: Uuid,
        update: &SubscriptionGroupUpdate,
    ) -> Result<Option<SubscriptionGroup>, Error>;

    /// Returns whether the group existed.
    async fn delete(&self, user_id: Uuid, id: Uuid) -> Result<bool, Error>;

    /// Moves a group to the zero-based `index`; past the end moves it to the end. Returns whether
    /// the group exists.
    async fn move_to(&self, user_id: Uuid, id: Uuid, index: u32) -> Result<bool, Error>;

    /// Adds a subscribed channel to a group. Idempotent.
    async fn add_channel(
        &self,
        user_id: Uuid,
        id: Uuid,
        channel_id: &str,
    ) -> Result<GroupChannelOutcome, Error>;

    /// Returns whether the group exists; succeeds if the channel was not in it, too.
    async fn remove_channel(
        &self,
        user_id: Uuid,
        id: Uuid,
        channel_id: &str,
    ) -> Result<bool, Error>;
}

#[derive(Debug, Clone)]
pub struct PgSubscriptionGroupRepository {
    pool: PgPool,
}

impl PgSubscriptionGroupRepository {
    pub fn new(pool: PgPool) -> Self {
        Self { pool }
    }
}

const GROUP_SELECT: &str = "SELECT g.id, g.name, g.show_on_home, \
        ARRAY(SELECT m.channel_id FROM subscription_group_channels m \
            WHERE m.group_id = g.id ORDER BY m.channel_id) AS channel_ids \
     FROM subscription_groups g";

fn name_taken(err: sqlx::Error) -> Error {
    match &err {
        sqlx::Error::Database(db) if db.constraint() == Some(UNIQUE_NAME) => Error::NameTaken,
        _ => err.into(),
    }
}

/// Serializes changes to a user's groups, so positions and the limit hold.
async fn lock_user(tx: &mut sqlx::PgConnection, user_id: Uuid) -> Result<(), Error> {
    sqlx::query("SELECT 1 FROM users WHERE id = $1 FOR NO KEY UPDATE")
        .bind(user_id)
        .execute(&mut *tx)
        .await?;
    Ok(())
}

async fn fetch_group(
    conn: &mut sqlx::PgConnection,
    user_id: Uuid,
    id: Uuid,
) -> Result<Option<SubscriptionGroup>, Error> {
    Ok(
        sqlx::query_as::<_, SubscriptionGroup>(sqlx::AssertSqlSafe(format!(
            "{GROUP_SELECT} WHERE g.id = $1 AND g.user_id = $2"
        )))
        .bind(id)
        .bind(user_id)
        .fetch_optional(&mut *conn)
        .await?,
    )
}

async fn group_exists(
    conn: &mut sqlx::PgConnection,
    user_id: Uuid,
    id: Uuid,
) -> Result<bool, Error> {
    Ok(sqlx::query_scalar(
        "SELECT EXISTS (SELECT 1 FROM subscription_groups WHERE id = $1 AND user_id = $2)",
    )
    .bind(id)
    .bind(user_id)
    .fetch_one(&mut *conn)
    .await?)
}

#[async_trait::async_trait]
impl SubscriptionGroupRepository for PgSubscriptionGroupRepository {
    async fn list(&self, user_id: Uuid) -> Result<Vec<SubscriptionGroup>, Error> {
        Ok(
            sqlx::query_as::<_, SubscriptionGroup>(sqlx::AssertSqlSafe(format!(
                "{GROUP_SELECT} WHERE g.user_id = $1 ORDER BY g.position, g.id"
            )))
            .bind(user_id)
            .fetch_all(&self.pool)
            .await?,
        )
    }

    async fn get(&self, user_id: Uuid, id: Uuid) -> Result<Option<SubscriptionGroup>, Error> {
        let mut conn = self.pool.acquire().await?;
        fetch_group(&mut conn, user_id, id).await
    }

    async fn create(
        &self,
        user_id: Uuid,
        group: &NewSubscriptionGroup,
        max_groups: u32,
    ) -> Result<SubscriptionGroup, Error> {
        let mut tx = self.pool.begin().await?;
        lock_user(&mut tx, user_id).await?;
        let count: i64 =
            sqlx::query_scalar("SELECT count(*) FROM subscription_groups WHERE user_id = $1")
                .bind(user_id)
                .fetch_one(&mut *tx)
                .await?;
        if count >= i64::from(max_groups) {
            return Err(Error::LimitReached);
        }
        let id: Uuid = sqlx::query_scalar(
            "INSERT INTO subscription_groups (user_id, name, position, show_on_home) \
             VALUES ($1, $2, $3, $4) RETURNING id",
        )
        .bind(user_id)
        .bind(&group.name)
        .bind(i32::try_from(count).unwrap_or(i32::MAX))
        .bind(group.show_on_home)
        .fetch_one(&mut *tx)
        .await
        .map_err(name_taken)?;
        if !group.channel_ids.is_empty() {
            sqlx::query(
                "INSERT INTO subscription_group_channels (group_id, user_id, channel_id) \
                 SELECT $1, s.user_id, s.channel_id FROM subscriptions s \
                 WHERE s.user_id = $2 AND s.channel_id = ANY ($3) \
                 ON CONFLICT DO NOTHING",
            )
            .bind(id)
            .bind(user_id)
            .bind(&group.channel_ids)
            .execute(&mut *tx)
            .await?;
        }
        let created = fetch_group(&mut tx, user_id, id)
            .await?
            .ok_or(Error::Sqlx(sqlx::Error::RowNotFound))?;
        tx.commit().await?;
        Ok(created)
    }

    async fn update(
        &self,
        user_id: Uuid,
        id: Uuid,
        update: &SubscriptionGroupUpdate,
    ) -> Result<Option<SubscriptionGroup>, Error> {
        let mut tx = self.pool.begin().await?;
        let updated = sqlx::query(
            "UPDATE subscription_groups SET \
                name = COALESCE($3, name), \
                show_on_home = COALESCE($4, show_on_home) \
             WHERE id = $1 AND user_id = $2",
        )
        .bind(id)
        .bind(user_id)
        .bind(update.name.as_deref())
        .bind(update.show_on_home)
        .execute(&mut *tx)
        .await
        .map_err(name_taken)?
        .rows_affected()
            > 0;
        if !updated {
            return Ok(None);
        }
        let group = fetch_group(&mut tx, user_id, id).await?;
        tx.commit().await?;
        Ok(group)
    }

    async fn delete(&self, user_id: Uuid, id: Uuid) -> Result<bool, Error> {
        let mut tx = self.pool.begin().await?;
        lock_user(&mut tx, user_id).await?;
        let position: Option<i32> = sqlx::query_scalar(
            "DELETE FROM subscription_groups WHERE id = $1 AND user_id = $2 RETURNING position",
        )
        .bind(id)
        .bind(user_id)
        .fetch_optional(&mut *tx)
        .await?;
        let Some(position) = position else {
            return Ok(false);
        };
        sqlx::query(
            "UPDATE subscription_groups SET position = position - 1 \
             WHERE user_id = $1 AND position > $2",
        )
        .bind(user_id)
        .bind(position)
        .execute(&mut *tx)
        .await?;
        tx.commit().await?;
        Ok(true)
    }

    async fn move_to(&self, user_id: Uuid, id: Uuid, index: u32) -> Result<bool, Error> {
        let mut tx = self.pool.begin().await?;
        lock_user(&mut tx, user_id).await?;
        let row: Option<(i32, i64)> = sqlx::query_as(
            "SELECT position, (SELECT count(*) FROM subscription_groups WHERE user_id = $2) \
             FROM subscription_groups WHERE id = $1 AND user_id = $2",
        )
        .bind(id)
        .bind(user_id)
        .fetch_optional(&mut *tx)
        .await?;
        let Some((from, count)) = row else {
            return Ok(false);
        };
        let last = i32::try_from(count - 1).unwrap_or(i32::MAX);
        let to = i32::try_from(index).unwrap_or(i32::MAX).min(last);
        if from != to {
            // Shifts the groups between the old and new position by one towards the gap.
            sqlx::query(
                "UPDATE subscription_groups SET position = CASE \
                    WHEN id = $2 THEN $4 \
                    WHEN $3 < $4 THEN position - 1 \
                    ELSE position + 1 END \
                 WHERE user_id = $1 AND position BETWEEN least($3, $4) AND greatest($3, $4)",
            )
            .bind(user_id)
            .bind(id)
            .bind(from)
            .bind(to)
            .execute(&mut *tx)
            .await?;
        }
        tx.commit().await?;
        Ok(true)
    }

    async fn add_channel(
        &self,
        user_id: Uuid,
        id: Uuid,
        channel_id: &str,
    ) -> Result<GroupChannelOutcome, Error> {
        let mut tx = self.pool.begin().await?;
        if !group_exists(&mut tx, user_id, id).await? {
            return Ok(GroupChannelOutcome::GroupNotFound);
        }
        let subscribed: bool = sqlx::query_scalar(
            "SELECT EXISTS (SELECT 1 FROM subscriptions WHERE user_id = $1 AND channel_id = $2)",
        )
        .bind(user_id)
        .bind(channel_id)
        .fetch_one(&mut *tx)
        .await?;
        if !subscribed {
            return Ok(GroupChannelOutcome::NotSubscribed);
        }
        let inserted = sqlx::query(
            "INSERT INTO subscription_group_channels (group_id, user_id, channel_id) \
             VALUES ($1, $2, $3) ON CONFLICT DO NOTHING",
        )
        .bind(id)
        .bind(user_id)
        .bind(channel_id)
        .execute(&mut *tx)
        .await;
        // An unsubscribe in between violates the foreign key.
        match inserted {
            Ok(_) => {}
            Err(sqlx::Error::Database(db)) if db.is_foreign_key_violation() => {
                return Ok(GroupChannelOutcome::NotSubscribed);
            }
            Err(err) => return Err(err.into()),
        }
        tx.commit().await?;
        Ok(GroupChannelOutcome::Done)
    }

    async fn remove_channel(
        &self,
        user_id: Uuid,
        id: Uuid,
        channel_id: &str,
    ) -> Result<bool, Error> {
        let mut tx = self.pool.begin().await?;
        if !group_exists(&mut tx, user_id, id).await? {
            return Ok(false);
        }
        sqlx::query(
            "DELETE FROM subscription_group_channels \
             WHERE group_id = $1 AND user_id = $2 AND channel_id = $3",
        )
        .bind(id)
        .bind(user_id)
        .bind(channel_id)
        .execute(&mut *tx)
        .await?;
        tx.commit().await?;
        Ok(true)
    }
}
