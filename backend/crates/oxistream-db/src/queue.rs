use std::collections::HashSet;

use sqlx::PgPool;
use uuid::Uuid;

use crate::Error;

/// A video in a user's up-next queue.
#[derive(Debug, Clone, PartialEq, Eq, sqlx::FromRow)]
pub struct QueueEntry {
    /// Tells entries of the same video apart; chosen by the client.
    pub key: String,
    pub video_id: String,
    pub title: String,
    pub channel_id: Option<String>,
    pub channel_name: Option<String>,
    pub duration_seconds: Option<i32>,
}

/// A user's up-next queue, in play order.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct Queue {
    /// Grows with every change; 0 for a queue that never changed.
    pub revision: i64,
    pub entries: Vec<QueueEntry>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum QueuePosition {
    Start,
    End,
}

/// A change to a queue. Changes refer to entries by key, so repeating one is harmless.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum QueueOp {
    /// Adds entries whose key is not queued yet; with `skip_queued` also leaves out videos
    /// that are already queued. Entries beyond the limit fall off the end.
    Add {
        entries: Vec<QueueEntry>,
        position: QueuePosition,
        skip_queued: bool,
    },
    Remove {
        key: String,
    },
    /// Moves an entry to `index`, clamped to the end.
    Move {
        key: String,
        index: u32,
    },
    Clear,
}

impl QueueOp {
    /// The entries after the change, or `None` if nothing changes.
    pub fn apply(&self, entries: &[QueueEntry], max: usize) -> Option<Vec<QueueEntry>> {
        let next: Vec<QueueEntry> = match self {
            Self::Add {
                entries: added,
                position,
                skip_queued,
            } => {
                let mut keys: HashSet<&str> = entries.iter().map(|e| e.key.as_str()).collect();
                let videos: HashSet<&str> = entries.iter().map(|e| e.video_id.as_str()).collect();
                let added = added
                    .iter()
                    .filter(|e| !(*skip_queued && videos.contains(e.video_id.as_str())))
                    .filter(|e| keys.insert(e.key.as_str()))
                    .cloned();
                match position {
                    QueuePosition::Start => {
                        added.chain(entries.iter().cloned()).take(max).collect()
                    }
                    QueuePosition::End => entries.iter().cloned().chain(added).take(max).collect(),
                }
            }
            Self::Remove { key } => entries.iter().filter(|e| &e.key != key).cloned().collect(),
            Self::Move { key, index } => {
                let from = entries.iter().position(|e| &e.key == key)?;
                let mut next = entries.to_vec();
                let entry = next.remove(from);
                next.insert((*index as usize).min(next.len()), entry);
                next
            }
            Self::Clear => Vec::new(),
        };
        (next.as_slice() != entries).then_some(next)
    }
}

#[async_trait::async_trait]
pub trait QueueRepository: Send + Sync {
    async fn get(&self, user_id: Uuid) -> Result<Queue, Error>;

    /// Applies `op`, keeping at most `max` entries, and returns the queue after it.
    async fn apply(&self, user_id: Uuid, op: &QueueOp, max: usize) -> Result<Queue, Error>;
}

#[derive(Debug, Clone)]
pub struct PgQueueRepository {
    pool: PgPool,
}

impl PgQueueRepository {
    pub fn new(pool: PgPool) -> Self {
        Self { pool }
    }
}

const ENTRIES: &str = "SELECT key, video_id, title, channel_id, channel_name, duration_seconds \
     FROM queue_entries WHERE user_id = $1 ORDER BY position";

#[async_trait::async_trait]
impl QueueRepository for PgQueueRepository {
    async fn get(&self, user_id: Uuid) -> Result<Queue, Error> {
        let mut tx = self.pool.begin().await?;
        let revision: Option<i64> =
            sqlx::query_scalar("SELECT revision FROM queues WHERE user_id = $1")
                .bind(user_id)
                .fetch_optional(&mut *tx)
                .await?;
        let entries = sqlx::query_as(ENTRIES)
            .bind(user_id)
            .fetch_all(&mut *tx)
            .await?;
        tx.commit().await?;
        Ok(Queue {
            revision: revision.unwrap_or(0),
            entries,
        })
    }

    async fn apply(&self, user_id: Uuid, op: &QueueOp, max: usize) -> Result<Queue, Error> {
        let mut tx = self.pool.begin().await?;
        sqlx::query("INSERT INTO queues (user_id) VALUES ($1) ON CONFLICT (user_id) DO NOTHING")
            .bind(user_id)
            .execute(&mut *tx)
            .await?;
        let revision: i64 =
            sqlx::query_scalar("SELECT revision FROM queues WHERE user_id = $1 FOR UPDATE")
                .bind(user_id)
                .fetch_one(&mut *tx)
                .await?;
        let entries: Vec<QueueEntry> = sqlx::query_as(ENTRIES)
            .bind(user_id)
            .fetch_all(&mut *tx)
            .await?;
        let Some(next) = op.apply(&entries, max) else {
            tx.commit().await?;
            return Ok(Queue { revision, entries });
        };
        sqlx::query("DELETE FROM queue_entries WHERE user_id = $1")
            .bind(user_id)
            .execute(&mut *tx)
            .await?;
        let column = |f: fn(&QueueEntry) -> Option<String>| next.iter().map(f).collect::<Vec<_>>();
        sqlx::query(
            "INSERT INTO queue_entries \
                (user_id, position, key, video_id, title, channel_id, channel_name, duration_seconds) \
             SELECT $1, e.position - 1, e.key, e.video_id, e.title, e.channel_id, e.channel_name, \
                e.duration_seconds \
             FROM unnest($2::text[], $3::text[], $4::text[], $5::text[], $6::text[], $7::int[]) \
                WITH ORDINALITY AS e(key, video_id, title, channel_id, channel_name, \
                    duration_seconds, position)",
        )
        .bind(user_id)
        .bind(column(|e| Some(e.key.clone())))
        .bind(column(|e| Some(e.video_id.clone())))
        .bind(column(|e| Some(e.title.clone())))
        .bind(column(|e| e.channel_id.clone()))
        .bind(column(|e| e.channel_name.clone()))
        .bind(next.iter().map(|e| e.duration_seconds).collect::<Vec<_>>())
        .execute(&mut *tx)
        .await?;
        let revision: i64 = sqlx::query_scalar(
            "UPDATE queues SET revision = revision + 1, updated_at = now() \
             WHERE user_id = $1 RETURNING revision",
        )
        .bind(user_id)
        .fetch_one(&mut *tx)
        .await?;
        tx.commit().await?;
        Ok(Queue {
            revision,
            entries: next,
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn entry(key: &str, video: &str) -> QueueEntry {
        QueueEntry {
            key: key.into(),
            video_id: video.into(),
            title: video.into(),
            channel_id: None,
            channel_name: None,
            duration_seconds: None,
        }
    }

    fn keys(entries: &[QueueEntry]) -> Vec<&str> {
        entries.iter().map(|e| e.key.as_str()).collect()
    }

    fn add(entries: Vec<QueueEntry>, position: QueuePosition, skip_queued: bool) -> QueueOp {
        QueueOp::Add {
            entries,
            position,
            skip_queued,
        }
    }

    #[test]
    fn adds_new_keys_at_either_end() {
        let queue = [entry("a", "v1"), entry("b", "v2")];
        let end = add(
            vec![entry("c", "v3"), entry("a", "v9")],
            QueuePosition::End,
            false,
        );
        assert_eq!(keys(&end.apply(&queue, 10).unwrap()), ["a", "b", "c"]);
        let start = add(vec![entry("c", "v1")], QueuePosition::Start, false);
        assert_eq!(keys(&start.apply(&queue, 10).unwrap()), ["c", "a", "b"]);
        let again = add(vec![entry("a", "v1")], QueuePosition::End, false);
        assert_eq!(again.apply(&queue, 10), None, "known keys are a no-op");
    }

    #[test]
    fn merging_skips_queued_videos_and_duplicate_keys() {
        let queue = [entry("a", "v1")];
        let merge = add(
            vec![entry("b", "v1"), entry("c", "v2"), entry("c", "v3")],
            QueuePosition::End,
            true,
        );
        assert_eq!(keys(&merge.apply(&queue, 10).unwrap()), ["a", "c"]);
    }

    #[test]
    fn keeps_the_limit() {
        let queue = [entry("a", "v1"), entry("b", "v2")];
        let end = add(vec![entry("c", "v3")], QueuePosition::End, false);
        assert_eq!(
            end.apply(&queue, 2),
            None,
            "a full queue takes nothing at the end"
        );
        let start = add(vec![entry("c", "v3")], QueuePosition::Start, false);
        assert_eq!(keys(&start.apply(&queue, 2).unwrap()), ["c", "a"]);
    }

    #[test]
    fn removes_moves_and_clears_by_key() {
        let queue = [entry("a", "v1"), entry("b", "v2"), entry("c", "v3")];
        let remove = QueueOp::Remove { key: "b".into() };
        assert_eq!(keys(&remove.apply(&queue, 10).unwrap()), ["a", "c"]);
        let gone = QueueOp::Remove { key: "x".into() };
        assert_eq!(gone.apply(&queue, 10), None);
        let to_end = QueueOp::Move {
            key: "a".into(),
            index: 99,
        };
        assert_eq!(keys(&to_end.apply(&queue, 10).unwrap()), ["b", "c", "a"]);
        let to_top = QueueOp::Move {
            key: "c".into(),
            index: 0,
        };
        assert_eq!(keys(&to_top.apply(&queue, 10).unwrap()), ["c", "a", "b"]);
        let stay = QueueOp::Move {
            key: "a".into(),
            index: 0,
        };
        assert_eq!(stay.apply(&queue, 10), None);
        assert_eq!(QueueOp::Clear.apply(&queue, 10), Some(Vec::new()));
        assert_eq!(QueueOp::Clear.apply(&[], 10), None);
    }
}
