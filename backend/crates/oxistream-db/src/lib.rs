//! Database access for Oxistream.

use std::time::Duration;

pub use sqlx::PgPool;
use sqlx::migrate::Migrator;
use sqlx::postgres::PgPoolOptions;

pub static MIGRATOR: Migrator = sqlx::migrate!("../../migrations");

#[derive(Debug, thiserror::Error)]
pub enum Error {
    #[error("database error")]
    Sqlx(#[from] sqlx::Error),
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
