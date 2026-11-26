use sqlx::{PgPool, Postgres, Transaction};

use crate::Error;

/// Keys of the Postgres advisory locks Oxistream takes.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[repr(i64)]
pub enum LockKey {
    FeedWorker = 0x6f78_6973_0001,
}

/// A held transaction-level advisory lock. Dropping it rolls back the transaction, which
/// releases the lock; so does a lost connection.
pub struct AdvisoryLock {
    tx: Transaction<'static, Postgres>,
}

impl AdvisoryLock {
    /// Takes the lock if no other session holds it.
    pub async fn try_acquire(pool: &PgPool, key: LockKey) -> Result<Option<Self>, Error> {
        let mut tx = pool.begin().await?;
        let acquired: bool = sqlx::query_scalar("SELECT pg_try_advisory_xact_lock($1)")
            .bind(key as i64)
            .fetch_one(&mut *tx)
            .await?;
        if !acquired {
            tx.rollback().await?;
            return Ok(None);
        }
        Ok(Some(Self { tx }))
    }

    pub async fn release(self) -> Result<(), Error> {
        self.tx.rollback().await?;
        Ok(())
    }
}
