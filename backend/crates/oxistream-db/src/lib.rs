//! Database access for Oxistream.

use std::time::Duration;

pub use sqlx::PgPool;
use sqlx::migrate::Migrator;
use sqlx::postgres::PgPoolOptions;

mod blocks;
mod follows;
mod history;
mod likes;
mod lock;
mod playlists;
mod queue;
mod rooms;
mod sessions;
mod settings;
mod sponsorblock;
mod subscriptions;
mod users;
mod videos;

pub use blocks::{ChannelBlock, ChannelBlockRepository, NewChannelBlock, PgChannelBlockRepository};
pub use follows::{
    FollowedPlaylist, PgPlaylistFollowRepository, PgYouTubePlaylistRepository,
    PlaylistFollowRepository, YouTubePlaylist, YouTubePlaylistItem, YouTubePlaylistItemUpsert,
    YouTubePlaylistRepository, YouTubePlaylistUpdate,
};
pub use history::{
    HistoryCursor, HistoryEntry, HistoryRepository, PgHistoryRepository, ProgressUpdate,
    WatchProgress,
};
pub use likes::{LikeCursor, LikeRepository, LikedVideo, PgLikeRepository};
pub use lock::{AdvisoryLock, LockKey};
pub use playlists::{
    NewPlaylist, PgPlaylistRepository, Playlist, PlaylistItem, PlaylistRepository, PlaylistUpdate,
    Visibility,
};
pub use queue::{PgQueueRepository, Queue, QueueEntry, QueueOp, QueuePosition, QueueRepository};
pub use rooms::{
    JoinOutcome, NewParticipant, OpOutcome, Participant, PgRoomRepository, ROOM_CHANNEL, Room,
    RoomEntry, RoomEvent, RoomListener, RoomOp, RoomRepository, RoomSnapshot, RoomSummary,
};
pub use sessions::{
    ActiveSession, BackchannelLogout, LogoutOutcome, NewSession, PgSessionRepository, Session,
    SessionRepository, TokenHash,
};
pub use settings::{
    PgUserSettingsRepository, SettingsUpdate, UserSettings, UserSettingsRepository,
};
pub use sponsorblock::{PgSponsorBlockSettingsRepository, SponsorBlockSettingsRepository};
pub use subscriptions::{
    Channel, ChannelRepository, ChannelUpdate, ChannelVideoUpsert, FeedCursor, FeedFilter,
    FeedStatus, FeedVideo, Image, ImportedChannel, PgChannelRepository, PgSubscriptionRepository,
    Subscription, SubscriptionRepository,
};
pub use users::{
    DeleteOutcome, Identity, NewUser, PgUserRepository, Role, User, UserCredentials,
    UserRepository, UsernameCandidates,
};
pub use uuid::Uuid;
pub use videos::{PgVideoRepository, Video, VideoRepository, VideoUpsert};

pub static MIGRATOR: Migrator = sqlx::migrate!("../../migrations");

#[derive(Debug, thiserror::Error)]
pub enum Error {
    #[error("database error")]
    Sqlx(#[from] sqlx::Error),
    #[error("username is already taken")]
    UsernameTaken,
    #[error("limit reached")]
    LimitReached,
    #[error("migration failed")]
    Migrate(#[from] sqlx::migrate::MigrateError),
}

#[derive(Debug, Clone)]
pub struct PoolOptions {
    pub max_connections: u32,
    pub acquire_timeout: Duration,
}

impl Default for PoolOptions {
    fn default() -> Self {
        Self {
            max_connections: 10,
            acquire_timeout: Duration::from_secs(5),
        }
    }
}

impl PoolOptions {
    fn builder(&self) -> PgPoolOptions {
        PgPoolOptions::new()
            .max_connections(self.max_connections)
            .acquire_timeout(self.acquire_timeout)
    }
}

pub async fn connect(url: &str, options: &PoolOptions) -> Result<PgPool, Error> {
    Ok(options.builder().connect(url).await?)
}

/// Creates a pool without opening a connection until first use.
pub fn connect_lazy(url: &str, options: &PoolOptions) -> Result<PgPool, Error> {
    Ok(options.builder().connect_lazy(url)?)
}

pub async fn migrate(pool: &PgPool) -> Result<(), Error> {
    MIGRATOR.run(pool).await?;
    Ok(())
}

pub async fn ping(pool: &PgPool) -> Result<(), Error> {
    sqlx::query("SELECT 1").execute(pool).await?;
    Ok(())
}
