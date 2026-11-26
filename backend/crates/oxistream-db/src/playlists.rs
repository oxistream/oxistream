use std::fmt;
use std::str::FromStr;

use sqlx::PgPool;
use time::OffsetDateTime;
use uuid::Uuid;

use crate::{Error, Video, VideoUpsert};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Visibility {
    /// Only the owner can see it.
    Private,
    /// Anyone with the link can see it.
    Unlisted,
}

impl Visibility {
    pub fn as_str(self) -> &'static str {
        match self {
            Self::Private => "private",
            Self::Unlisted => "unlisted",
        }
    }
}

impl fmt::Display for Visibility {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(self.as_str())
    }
}

impl FromStr for Visibility {
    type Err = String;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s {
            "private" => Ok(Self::Private),
            "unlisted" => Ok(Self::Unlisted),
            _ => Err(format!("unknown visibility {s:?}")),
        }
    }
}

impl TryFrom<String> for Visibility {
    type Error = String;

    fn try_from(s: String) -> Result<Self, Self::Error> {
        s.parse()
    }
}

#[derive(Debug, Clone, PartialEq, Eq, sqlx::FromRow)]
pub struct Playlist {
    pub id: String,
    pub user_id: Uuid,
    /// Username of the owner.
    pub owner: String,
    pub title: String,
    pub description: String,
    #[sqlx(try_from = "String")]
    pub visibility: Visibility,
    /// Where the owner left off playing; always an item of the playlist.
    pub last_video_id: Option<String>,
    pub item_count: i64,
    pub first_video_id: Option<String>,
    /// Whether the video asked for in [`PlaylistRepository::list`] is in the playlist.
    pub contains_video: Option<bool>,
    pub created_at: OffsetDateTime,
    pub updated_at: OffsetDateTime,
}

#[derive(Debug, Clone, PartialEq, Eq, sqlx::FromRow)]
pub struct PlaylistItem {
    /// Zero-based.
    pub position: i32,
    pub added_at: OffsetDateTime,
    #[sqlx(flatten)]
    pub video: Video,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct NewPlaylist {
    pub id: String,
    pub title: String,
    pub description: String,
    pub visibility: Visibility,
}

/// Changed fields; `None` keeps the stored value.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct PlaylistUpdate {
    pub title: Option<String>,
    pub description: Option<String>,
    pub visibility: Option<Visibility>,
}

/// Mutations take the owner and treat other users' playlists as nonexistent.
#[async_trait::async_trait]
pub trait PlaylistRepository: Send + Sync {
    /// Fails with [`Error::LimitReached`] if the user already has `max_playlists`.
    async fn create(
        &self,
        user_id: Uuid,
        playlist: &NewPlaylist,
        max_playlists: u32,
    ) -> Result<Playlist, Error>;

    /// Like [`PlaylistRepository::create`], with `videos` as items in order. Videos not cached
    /// yet are stored; known ones keep their cached metadata. Duplicates keep their first place.
    async fn create_with_items(
        &self,
        user_id: Uuid,
        playlist: &NewPlaylist,
        videos: &[VideoUpsert],
        max_playlists: u32,
    ) -> Result<Playlist, Error>;

    /// A user's playlists, most recently changed first. With `video_id`, each one tells
    /// whether it contains that video.
    async fn list(&self, user_id: Uuid, video_id: Option<&str>) -> Result<Vec<Playlist>, Error>;

    /// Any user's playlist; visibility is up to the caller.
    async fn get(&self, id: &str) -> Result<Option<Playlist>, Error>;

    /// Items in playlist order.
    async fn items(&self, id: &str) -> Result<Vec<PlaylistItem>, Error>;

    async fn update(
        &self,
        user_id: Uuid,
        id: &str,
        update: &PlaylistUpdate,
    ) -> Result<Option<Playlist>, Error>;

    /// Returns whether the playlist existed.
    async fn delete(&self, user_id: Uuid, id: &str) -> Result<bool, Error>;

    /// Appends a cached video. `None` if the playlist does not exist, else whether the video
    /// is new to it. Fails with [`Error::LimitReached`] at `max_items`.
    async fn add_item(
        &self,
        user_id: Uuid,
        id: &str,
        video_id: &str,
        max_items: u32,
    ) -> Result<Option<bool>, Error>;

    /// `None` if the playlist does not exist, else whether the video was in it.
    async fn remove_item(
        &self,
        user_id: Uuid,
        id: &str,
        video_id: &str,
    ) -> Result<Option<bool>, Error>;

    /// Moves an item to `index`, clamped to the end. `None` if the playlist does not exist,
    /// else whether the video is in it.
    async fn move_item(
        &self,
        user_id: Uuid,
        id: &str,
        video_id: &str,
        index: u32,
    ) -> Result<Option<bool>, Error>;

    /// Remembers the item the owner is playing. Returns whether playlist and item exist.
    async fn set_last_played(&self, user_id: Uuid, id: &str, video_id: &str)
    -> Result<bool, Error>;
}

#[derive(Debug, Clone)]
pub struct PgPlaylistRepository {
    pool: PgPool,
}

impl PgPlaylistRepository {
    pub fn new(pool: PgPool) -> Self {
        Self { pool }
    }
}

/// Expects the video to match against as `$1`.
const PLAYLIST_SELECT: &str = "SELECT p.id, p.user_id, u.username AS owner, p.title, \
        p.description, p.visibility, p.last_video_id, p.created_at, p.updated_at, \
        (SELECT count(*) FROM playlist_items i WHERE i.playlist_id = p.id) AS item_count, \
        (SELECT i.video_id FROM playlist_items i WHERE i.playlist_id = p.id \
            ORDER BY i.position LIMIT 1) AS first_video_id, \
        CASE WHEN $1::text IS NULL THEN NULL ELSE EXISTS \
            (SELECT 1 FROM playlist_items i WHERE i.playlist_id = p.id AND i.video_id = $1) \
        END AS contains_video \
    FROM playlists p JOIN users u ON u.id = p.user_id";

/// Locks an owned playlist for the rest of the transaction; `false` if there is none.
async fn lock_playlist(
    tx: &mut sqlx::PgConnection,
    user_id: Uuid,
    id: &str,
) -> Result<bool, Error> {
    Ok(
        sqlx::query("SELECT 1 FROM playlists WHERE id = $1 AND user_id = $2 FOR UPDATE")
            .bind(id)
            .bind(user_id)
            .fetch_optional(&mut *tx)
            .await?
            .is_some(),
    )
}

/// Inserts a playlist unless the user already has `max_playlists`.
async fn insert_playlist(
    tx: &mut sqlx::PgConnection,
    user_id: Uuid,
    playlist: &NewPlaylist,
    max_playlists: u32,
) -> Result<(), Error> {
    // Serializes concurrent creations of one user, so the limit holds.
    sqlx::query("SELECT 1 FROM users WHERE id = $1 FOR NO KEY UPDATE")
        .bind(user_id)
        .execute(&mut *tx)
        .await?;
    let count: i64 = sqlx::query_scalar("SELECT count(*) FROM playlists WHERE user_id = $1")
        .bind(user_id)
        .fetch_one(&mut *tx)
        .await?;
    if count >= i64::from(max_playlists) {
        return Err(Error::LimitReached);
    }
    sqlx::query(
        "INSERT INTO playlists (id, user_id, title, description, visibility) \
         VALUES ($1, $2, $3, $4, $5)",
    )
    .bind(&playlist.id)
    .bind(user_id)
    .bind(&playlist.title)
    .bind(&playlist.description)
    .bind(playlist.visibility.as_str())
    .execute(&mut *tx)
    .await?;
    Ok(())
}

async fn select_playlist(tx: &mut sqlx::PgConnection, id: &str) -> Result<Playlist, Error> {
    Ok(sqlx::query_as::<_, Playlist>(sqlx::AssertSqlSafe(format!(
        "{PLAYLIST_SELECT} WHERE p.id = $2"
    )))
    .bind(None::<&str>)
    .bind(id)
    .fetch_one(&mut *tx)
    .await?)
}

async fn touch(tx: &mut sqlx::PgConnection, id: &str) -> Result<(), Error> {
    sqlx::query("UPDATE playlists SET updated_at = now() WHERE id = $1")
        .bind(id)
        .execute(&mut *tx)
        .await?;
    Ok(())
}

#[async_trait::async_trait]
impl PlaylistRepository for PgPlaylistRepository {
    async fn create(
        &self,
        user_id: Uuid,
        playlist: &NewPlaylist,
        max_playlists: u32,
    ) -> Result<Playlist, Error> {
        let mut tx = self.pool.begin().await?;
        insert_playlist(&mut tx, user_id, playlist, max_playlists).await?;
        let created = select_playlist(&mut tx, &playlist.id).await?;
        tx.commit().await?;
        Ok(created)
    }

    async fn create_with_items(
        &self,
        user_id: Uuid,
        playlist: &NewPlaylist,
        videos: &[VideoUpsert],
        max_playlists: u32,
    ) -> Result<Playlist, Error> {
        let mut seen = std::collections::HashSet::new();
        let videos: Vec<_> = videos
            .iter()
            .filter(|v| seen.insert(v.id.as_str()))
            .collect();
        let ids: Vec<&str> = videos.iter().map(|v| v.id.as_str()).collect();
        let titles: Vec<&str> = videos.iter().map(|v| v.title.as_str()).collect();
        let channel_ids: Vec<&str> = videos.iter().map(|v| v.channel_id.as_str()).collect();
        let channel_names: Vec<&str> = videos.iter().map(|v| v.channel_name.as_str()).collect();
        let durations: Vec<Option<i32>> = videos.iter().map(|v| v.duration_seconds).collect();
        let published: Vec<Option<OffsetDateTime>> =
            videos.iter().map(|v| v.published_at).collect();
        let thumbnails: Vec<Option<&str>> =
            videos.iter().map(|v| v.thumbnail_url.as_deref()).collect();

        let mut tx = self.pool.begin().await?;
        insert_playlist(&mut tx, user_id, playlist, max_playlists).await?;
        sqlx::query(
            "INSERT INTO videos \
                (id, title, channel_id, channel_name, duration_seconds, published_at, thumbnail_url) \
             SELECT * FROM UNNEST($1::text[], $2::text[], $3::text[], $4::text[], $5::int[], \
                $6::timestamptz[], $7::text[]) \
             ON CONFLICT (id) DO NOTHING",
        )
        .bind(&ids)
        .bind(&titles)
        .bind(&channel_ids)
        .bind(&channel_names)
        .bind(&durations)
        .bind(&published)
        .bind(&thumbnails)
        .execute(&mut *tx)
        .await?;
        sqlx::query(
            "INSERT INTO playlist_items (playlist_id, video_id, position) \
             SELECT $1, t.id, t.ord - 1 FROM UNNEST($2::text[]) WITH ORDINALITY AS t (id, ord)",
        )
        .bind(&playlist.id)
        .bind(&ids)
        .execute(&mut *tx)
        .await?;
        let created = select_playlist(&mut tx, &playlist.id).await?;
        tx.commit().await?;
        Ok(created)
    }

    async fn list(&self, user_id: Uuid, video_id: Option<&str>) -> Result<Vec<Playlist>, Error> {
        Ok(sqlx::query_as::<_, Playlist>(sqlx::AssertSqlSafe(format!(
            "{PLAYLIST_SELECT} WHERE p.user_id = $2 ORDER BY p.updated_at DESC, p.id"
        )))
        .bind(video_id)
        .bind(user_id)
        .fetch_all(&self.pool)
        .await?)
    }

    async fn get(&self, id: &str) -> Result<Option<Playlist>, Error> {
        Ok(sqlx::query_as::<_, Playlist>(sqlx::AssertSqlSafe(format!(
            "{PLAYLIST_SELECT} WHERE p.id = $2"
        )))
        .bind(None::<&str>)
        .bind(id)
        .fetch_optional(&self.pool)
        .await?)
    }

    async fn items(&self, id: &str) -> Result<Vec<PlaylistItem>, Error> {
        Ok(sqlx::query_as::<_, PlaylistItem>(
            "SELECT i.position, i.added_at, v.id, v.title, v.channel_id, v.channel_name, \
                v.duration_seconds, v.published_at, v.thumbnail_url, v.updated_at \
             FROM playlist_items i JOIN videos v ON v.id = i.video_id \
             WHERE i.playlist_id = $1 ORDER BY i.position",
        )
        .bind(id)
        .fetch_all(&self.pool)
        .await?)
    }

    async fn update(
        &self,
        user_id: Uuid,
        id: &str,
        update: &PlaylistUpdate,
    ) -> Result<Option<Playlist>, Error> {
        let updated = sqlx::query(
            "UPDATE playlists SET \
                title = COALESCE($3, title), \
                description = COALESCE($4, description), \
                visibility = COALESCE($5, visibility), \
                updated_at = now() \
             WHERE id = $1 AND user_id = $2",
        )
        .bind(id)
        .bind(user_id)
        .bind(&update.title)
        .bind(&update.description)
        .bind(update.visibility.map(Visibility::as_str))
        .execute(&self.pool)
        .await?
        .rows_affected()
            > 0;
        if !updated {
            return Ok(None);
        }
        self.get(id).await
    }

    async fn delete(&self, user_id: Uuid, id: &str) -> Result<bool, Error> {
        Ok(
            sqlx::query("DELETE FROM playlists WHERE id = $1 AND user_id = $2")
                .bind(id)
                .bind(user_id)
                .execute(&self.pool)
                .await?
                .rows_affected()
                > 0,
        )
    }

    async fn add_item(
        &self,
        user_id: Uuid,
        id: &str,
        video_id: &str,
        max_items: u32,
    ) -> Result<Option<bool>, Error> {
        let mut tx = self.pool.begin().await?;
        if !lock_playlist(&mut tx, user_id, id).await? {
            return Ok(None);
        }
        let (count, present): (i64, bool) = sqlx::query_as(
            "SELECT count(*), coalesce(bool_or(video_id = $2), false) \
             FROM playlist_items WHERE playlist_id = $1",
        )
        .bind(id)
        .bind(video_id)
        .fetch_one(&mut *tx)
        .await?;
        if present {
            return Ok(Some(false));
        }
        if count >= i64::from(max_items) {
            return Err(Error::LimitReached);
        }
        sqlx::query(
            "INSERT INTO playlist_items (playlist_id, video_id, position) VALUES ($1, $2, $3)",
        )
        .bind(id)
        .bind(video_id)
        .bind(i32::try_from(count).unwrap_or(i32::MAX))
        .execute(&mut *tx)
        .await?;
        touch(&mut tx, id).await?;
        tx.commit().await?;
        Ok(Some(true))
    }

    async fn remove_item(
        &self,
        user_id: Uuid,
        id: &str,
        video_id: &str,
    ) -> Result<Option<bool>, Error> {
        let mut tx = self.pool.begin().await?;
        if !lock_playlist(&mut tx, user_id, id).await? {
            return Ok(None);
        }
        let position: Option<i32> = sqlx::query_scalar(
            "DELETE FROM playlist_items WHERE playlist_id = $1 AND video_id = $2 \
             RETURNING position",
        )
        .bind(id)
        .bind(video_id)
        .fetch_optional(&mut *tx)
        .await?;
        let Some(position) = position else {
            return Ok(Some(false));
        };
        sqlx::query(
            "UPDATE playlist_items SET position = position - 1 \
             WHERE playlist_id = $1 AND position > $2",
        )
        .bind(id)
        .bind(position)
        .execute(&mut *tx)
        .await?;
        sqlx::query(
            "UPDATE playlists SET updated_at = now(), \
                last_video_id = NULLIF(last_video_id, $2) \
             WHERE id = $1",
        )
        .bind(id)
        .bind(video_id)
        .execute(&mut *tx)
        .await?;
        tx.commit().await?;
        Ok(Some(true))
    }

    async fn move_item(
        &self,
        user_id: Uuid,
        id: &str,
        video_id: &str,
        index: u32,
    ) -> Result<Option<bool>, Error> {
        let mut tx = self.pool.begin().await?;
        if !lock_playlist(&mut tx, user_id, id).await? {
            return Ok(None);
        }
        let row: Option<(i32, i64)> = sqlx::query_as(
            "SELECT position, (SELECT count(*) FROM playlist_items WHERE playlist_id = $1) \
             FROM playlist_items WHERE playlist_id = $1 AND video_id = $2",
        )
        .bind(id)
        .bind(video_id)
        .fetch_optional(&mut *tx)
        .await?;
        let Some((from, count)) = row else {
            return Ok(Some(false));
        };
        let last = i32::try_from(count - 1).unwrap_or(i32::MAX);
        let to = i32::try_from(index).unwrap_or(i32::MAX).min(last);
        if from != to {
            // Shifts the items between the old and new position by one towards the gap.
            sqlx::query(
                "UPDATE playlist_items SET position = CASE \
                    WHEN video_id = $2 THEN $4 \
                    WHEN $3 < $4 THEN position - 1 \
                    ELSE position + 1 END \
                 WHERE playlist_id = $1 AND position BETWEEN least($3, $4) AND greatest($3, $4)",
            )
            .bind(id)
            .bind(video_id)
            .bind(from)
            .bind(to)
            .execute(&mut *tx)
            .await?;
            touch(&mut tx, id).await?;
        }
        tx.commit().await?;
        Ok(Some(true))
    }

    async fn set_last_played(
        &self,
        user_id: Uuid,
        id: &str,
        video_id: &str,
    ) -> Result<bool, Error> {
        Ok(sqlx::query(
            "UPDATE playlists p SET last_video_id = $3 \
             WHERE p.id = $1 AND p.user_id = $2 AND EXISTS \
                (SELECT 1 FROM playlist_items i WHERE i.playlist_id = p.id AND i.video_id = $3)",
        )
        .bind(id)
        .bind(user_id)
        .bind(video_id)
        .execute(&self.pool)
        .await?
        .rows_affected()
            > 0)
    }
}
