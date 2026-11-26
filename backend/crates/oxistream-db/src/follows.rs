use sqlx::PgPool;
use sqlx::types::Json;
use time::OffsetDateTime;
use uuid::Uuid;

use crate::{Error, Image};

/// A YouTube playlist someone follows, with what the feed worker last saw of it.
#[derive(Debug, Clone, PartialEq, Eq, sqlx::FromRow)]
pub struct YouTubePlaylist {
    pub id: String,
    /// `None` until the playlist was fetched once.
    pub title: Option<String>,
    pub channel_id: Option<String>,
    pub channel_name: Option<String>,
    #[sqlx(json)]
    pub thumbnails: Vec<Image>,
    /// As reported by YouTube; may exceed the stored items.
    pub video_count: Option<i64>,
    /// Last successful fetch.
    pub fetched_at: Option<OffsetDateTime>,
    /// Last fetch attempt.
    pub checked_at: Option<OffsetDateTime>,
    pub last_error: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Eq, sqlx::FromRow)]
pub struct FollowedPlaylist {
    #[sqlx(flatten)]
    pub playlist: YouTubePlaylist,
    pub followed_at: OffsetDateTime,
    /// Stored items that appeared since the user last opened the playlist.
    pub new_count: i64,
}

/// A stored item of a followed playlist.
#[derive(Debug, Clone, PartialEq, Eq, sqlx::FromRow)]
pub struct YouTubePlaylistItem {
    pub position: i32,
    pub video_id: String,
    pub title: String,
    pub channel_id: Option<String>,
    pub channel_name: Option<String>,
    pub duration_seconds: Option<i32>,
    /// `None` for items present at the first fetch.
    pub first_seen_at: Option<OffsetDateTime>,
}

#[async_trait::async_trait]
pub trait PlaylistFollowRepository: Send + Sync {
    /// Follows a playlist, creating it if unknown. Returns the follow and whether it is new.
    async fn follow(
        &self,
        user_id: Uuid,
        playlist_id: &str,
    ) -> Result<(FollowedPlaylist, bool), Error>;

    /// Returns whether a follow existed.
    async fn unfollow(&self, user_id: Uuid, playlist_id: &str) -> Result<bool, Error>;

    async fn is_following(&self, user_id: Uuid, playlist_id: &str) -> Result<bool, Error>;

    /// All followed playlists of a user, ordered by title.
    async fn list(&self, user_id: Uuid) -> Result<Vec<FollowedPlaylist>, Error>;

    /// Marks all current items as seen. Returns whether the user follows the playlist.
    async fn mark_seen(&self, user_id: Uuid, playlist_id: &str) -> Result<bool, Error>;
}

/// Playlist metadata written by a successful fetch.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct YouTubePlaylistUpdate {
    pub id: String,
    pub title: String,
    pub channel_id: Option<String>,
    pub channel_name: Option<String>,
    pub thumbnails: Vec<Image>,
    pub video_count: Option<i64>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct YouTubePlaylistItemUpsert {
    pub video_id: String,
    pub title: String,
    pub channel_id: Option<String>,
    pub channel_name: Option<String>,
    pub duration_seconds: Option<i32>,
}

/// Feed worker side of followed playlists.
#[async_trait::async_trait]
pub trait YouTubePlaylistRepository: Send + Sync {
    async fn get(&self, id: &str) -> Result<Option<YouTubePlaylist>, Error>;

    /// Items in playlist order.
    async fn items(&self, id: &str) -> Result<Vec<YouTubePlaylistItem>, Error>;

    /// IDs of followed playlists not checked since `checked_before`, least recently
    /// checked first.
    async fn due(&self, checked_before: OffsetDateTime, limit: u32) -> Result<Vec<String>, Error>;

    /// Stores a successful fetch: metadata and the items in order, replacing the stored
    /// ones. A video listed twice keeps its first place.
    async fn store_fetch(
        &self,
        playlist: &YouTubePlaylistUpdate,
        items: &[YouTubePlaylistItemUpsert],
    ) -> Result<(), Error>;

    async fn record_failure(&self, id: &str, error: &str) -> Result<(), Error>;

    /// Deletes the items of playlists nobody follows anymore.
    async fn prune_unfollowed(&self) -> Result<u64, Error>;
}

#[derive(Debug, Clone)]
pub struct PgPlaylistFollowRepository {
    pool: PgPool,
}

impl PgPlaylistFollowRepository {
    pub fn new(pool: PgPool) -> Self {
        Self { pool }
    }
}

const FOLLOW_SELECT: &str = "SELECT p.id, p.title, p.channel_id, p.channel_name, p.thumbnails, \
        p.video_count, p.fetched_at, p.checked_at, p.last_error, f.created_at AS followed_at, \
        (SELECT count(*) FROM youtube_playlist_items i \
            WHERE i.playlist_id = p.id AND i.first_seen_at > f.seen_at) AS new_count \
    FROM playlist_follows f JOIN youtube_playlists p ON p.id = f.playlist_id";

#[async_trait::async_trait]
impl PlaylistFollowRepository for PgPlaylistFollowRepository {
    async fn follow(
        &self,
        user_id: Uuid,
        playlist_id: &str,
    ) -> Result<(FollowedPlaylist, bool), Error> {
        let mut tx = self.pool.begin().await?;
        sqlx::query("INSERT INTO youtube_playlists (id) VALUES ($1) ON CONFLICT (id) DO NOTHING")
            .bind(playlist_id)
            .execute(&mut *tx)
            .await?;
        let created = sqlx::query(
            "INSERT INTO playlist_follows (user_id, playlist_id) VALUES ($1, $2) \
             ON CONFLICT DO NOTHING",
        )
        .bind(user_id)
        .bind(playlist_id)
        .execute(&mut *tx)
        .await?
        .rows_affected()
            > 0;
        let follow = sqlx::query_as::<_, FollowedPlaylist>(sqlx::AssertSqlSafe(format!(
            "{FOLLOW_SELECT} WHERE f.user_id = $1 AND f.playlist_id = $2"
        )))
        .bind(user_id)
        .bind(playlist_id)
        .fetch_one(&mut *tx)
        .await?;
        tx.commit().await?;
        Ok((follow, created))
    }

    async fn unfollow(&self, user_id: Uuid, playlist_id: &str) -> Result<bool, Error> {
        Ok(
            sqlx::query("DELETE FROM playlist_follows WHERE user_id = $1 AND playlist_id = $2")
                .bind(user_id)
                .bind(playlist_id)
                .execute(&self.pool)
                .await?
                .rows_affected()
                > 0,
        )
    }

    async fn is_following(&self, user_id: Uuid, playlist_id: &str) -> Result<bool, Error> {
        Ok(sqlx::query_scalar(
            "SELECT EXISTS \
                (SELECT 1 FROM playlist_follows WHERE user_id = $1 AND playlist_id = $2)",
        )
        .bind(user_id)
        .bind(playlist_id)
        .fetch_one(&self.pool)
        .await?)
    }

    async fn list(&self, user_id: Uuid) -> Result<Vec<FollowedPlaylist>, Error> {
        Ok(
            sqlx::query_as::<_, FollowedPlaylist>(sqlx::AssertSqlSafe(format!(
                "{FOLLOW_SELECT} WHERE f.user_id = $1 ORDER BY lower(p.title) NULLS LAST, p.id"
            )))
            .bind(user_id)
            .fetch_all(&self.pool)
            .await?,
        )
    }

    async fn mark_seen(&self, user_id: Uuid, playlist_id: &str) -> Result<bool, Error> {
        Ok(sqlx::query(
            "UPDATE playlist_follows SET seen_at = now() WHERE user_id = $1 AND playlist_id = $2",
        )
        .bind(user_id)
        .bind(playlist_id)
        .execute(&self.pool)
        .await?
        .rows_affected()
            > 0)
    }
}

#[derive(Debug, Clone)]
pub struct PgYouTubePlaylistRepository {
    pool: PgPool,
}

impl PgYouTubePlaylistRepository {
    pub fn new(pool: PgPool) -> Self {
        Self { pool }
    }
}

#[async_trait::async_trait]
impl YouTubePlaylistRepository for PgYouTubePlaylistRepository {
    async fn get(&self, id: &str) -> Result<Option<YouTubePlaylist>, Error> {
        Ok(sqlx::query_as::<_, YouTubePlaylist>(
            "SELECT id, title, channel_id, channel_name, thumbnails, video_count, fetched_at, \
                checked_at, last_error \
             FROM youtube_playlists WHERE id = $1",
        )
        .bind(id)
        .fetch_optional(&self.pool)
        .await?)
    }

    async fn items(&self, id: &str) -> Result<Vec<YouTubePlaylistItem>, Error> {
        Ok(sqlx::query_as::<_, YouTubePlaylistItem>(
            "SELECT position, video_id, title, channel_id, channel_name, duration_seconds, \
                first_seen_at \
             FROM youtube_playlist_items WHERE playlist_id = $1 ORDER BY position",
        )
        .bind(id)
        .fetch_all(&self.pool)
        .await?)
    }

    async fn due(&self, checked_before: OffsetDateTime, limit: u32) -> Result<Vec<String>, Error> {
        Ok(sqlx::query_scalar(
            "SELECT p.id FROM youtube_playlists p \
             WHERE EXISTS (SELECT 1 FROM playlist_follows f WHERE f.playlist_id = p.id) \
                AND (p.checked_at IS NULL OR p.checked_at < $1) \
             ORDER BY p.checked_at NULLS FIRST, p.id \
             LIMIT $2",
        )
        .bind(checked_before)
        .bind(i64::from(limit))
        .fetch_all(&self.pool)
        .await?)
    }

    async fn store_fetch(
        &self,
        playlist: &YouTubePlaylistUpdate,
        items: &[YouTubePlaylistItemUpsert],
    ) -> Result<(), Error> {
        let mut seen = std::collections::HashSet::new();
        let items: Vec<_> = items
            .iter()
            .filter(|i| seen.insert(i.video_id.as_str()))
            .collect();
        let ids: Vec<&str> = items.iter().map(|i| i.video_id.as_str()).collect();
        let positions: Vec<i32> = (0..i32::try_from(items.len()).unwrap_or(i32::MAX)).collect();
        let titles: Vec<&str> = items.iter().map(|i| i.title.as_str()).collect();
        let channel_ids: Vec<Option<&str>> =
            items.iter().map(|i| i.channel_id.as_deref()).collect();
        let channel_names: Vec<Option<&str>> =
            items.iter().map(|i| i.channel_name.as_deref()).collect();
        let durations: Vec<Option<i32>> = items.iter().map(|i| i.duration_seconds).collect();

        let mut tx = self.pool.begin().await?;
        let previous: Option<Option<OffsetDateTime>> =
            sqlx::query_scalar("SELECT fetched_at FROM youtube_playlists WHERE id = $1 FOR UPDATE")
                .bind(&playlist.id)
                .fetch_optional(&mut *tx)
                .await?;
        let first_fetch = previous.flatten().is_none();
        sqlx::query(
            "INSERT INTO youtube_playlists \
                (id, title, channel_id, channel_name, thumbnails, video_count, fetched_at, \
                 checked_at) \
             VALUES ($1, $2, $3, $4, $5, $6, now(), now()) \
             ON CONFLICT (id) DO UPDATE SET \
                title = EXCLUDED.title, \
                channel_id = EXCLUDED.channel_id, \
                channel_name = EXCLUDED.channel_name, \
                thumbnails = EXCLUDED.thumbnails, \
                video_count = EXCLUDED.video_count, \
                fetched_at = now(), \
                checked_at = now(), \
                last_error = NULL",
        )
        .bind(&playlist.id)
        .bind(&playlist.title)
        .bind(&playlist.channel_id)
        .bind(&playlist.channel_name)
        .bind(Json(&playlist.thumbnails))
        .bind(playlist.video_count)
        .execute(&mut *tx)
        .await?;
        sqlx::query(
            "DELETE FROM youtube_playlist_items \
             WHERE playlist_id = $1 AND NOT (video_id = ANY ($2::text[]))",
        )
        .bind(&playlist.id)
        .bind(&ids)
        .execute(&mut *tx)
        .await?;
        sqlx::query(
            "INSERT INTO youtube_playlist_items \
                (playlist_id, video_id, position, title, channel_id, channel_name, \
                 duration_seconds, first_seen_at) \
             SELECT $1, t.*, CASE WHEN $8 THEN NULL ELSE now() END \
             FROM UNNEST($2::text[], $3::int[], $4::text[], $5::text[], $6::text[], $7::int[]) \
                AS t (video_id, position, title, channel_id, channel_name, duration_seconds) \
             ON CONFLICT (playlist_id, video_id) DO UPDATE SET \
                position = EXCLUDED.position, \
                title = EXCLUDED.title, \
                channel_id = EXCLUDED.channel_id, \
                channel_name = EXCLUDED.channel_name, \
                duration_seconds = COALESCE(EXCLUDED.duration_seconds, \
                    youtube_playlist_items.duration_seconds)",
        )
        .bind(&playlist.id)
        .bind(&ids)
        .bind(&positions)
        .bind(&titles)
        .bind(&channel_ids)
        .bind(&channel_names)
        .bind(&durations)
        .bind(first_fetch)
        .execute(&mut *tx)
        .await?;
        tx.commit().await?;
        Ok(())
    }

    async fn record_failure(&self, id: &str, error: &str) -> Result<(), Error> {
        sqlx::query(
            "UPDATE youtube_playlists SET checked_at = now(), last_error = $2 WHERE id = $1",
        )
        .bind(id)
        .bind(error)
        .execute(&self.pool)
        .await?;
        Ok(())
    }

    async fn prune_unfollowed(&self) -> Result<u64, Error> {
        let mut tx = self.pool.begin().await?;
        let deleted = sqlx::query(
            "DELETE FROM youtube_playlist_items i WHERE NOT EXISTS \
                (SELECT 1 FROM playlist_follows f WHERE f.playlist_id = i.playlist_id)",
        )
        .execute(&mut *tx)
        .await?
        .rows_affected();
        // Forces a fresh fetch, without new items, should someone follow again.
        sqlx::query(
            "UPDATE youtube_playlists p SET fetched_at = NULL, checked_at = NULL \
             WHERE p.checked_at IS NOT NULL AND NOT EXISTS \
                (SELECT 1 FROM playlist_follows f WHERE f.playlist_id = p.id)",
        )
        .execute(&mut *tx)
        .await?;
        tx.commit().await?;
        Ok(deleted)
    }
}
