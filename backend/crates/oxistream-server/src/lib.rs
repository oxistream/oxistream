//! Oxistream HTTP server.

use std::net::SocketAddr;
use std::sync::Arc;

use axum::Router;
use metrics_exporter_prometheus::PrometheusHandle;
use oxistream_db::{
    PgPool, PgSessionRepository, PgUserRepository, PgVideoRepository, VideoRepository,
};
use tokio::net::TcpListener;
use tower_http::compression::CompressionLayer;
use tower_http::compression::predicate::{DefaultPredicate, NotForContentType, Predicate};
use tower_http::trace::{DefaultMakeSpan, DefaultOnResponse, TraceLayer};
use tracing::Level;

pub mod api;
pub mod auth;
pub mod config;
mod ops;
pub mod proxy;
pub mod rate_limit;
mod spa;
pub mod telemetry;
pub mod youtube;

pub use config::Config;

#[derive(Clone)]
pub struct AppState {
    pub db: PgPool,
    pub metrics: PrometheusHandle,
    pub youtube: youtube::YouTube,
    pub videos: Arc<dyn VideoRepository>,
    pub proxy: proxy::Proxy,
    /// `None` disables rate limiting.
    pub rate_limit: Option<rate_limit::RateLimit>,
    pub auth: auth::Auth,
}

impl AppState {
    /// The URL clients use to load an upstream image. All image URLs in responses go through here.
    pub fn image_url(&self, upstream: &str) -> String {
        self.proxy.image_url(upstream)
    }

    /// Signed proxy URL of an adaptive stream; `track` is the audio track ID, if any.
    pub fn stream_url(&self, video_id: &str, itag: u32, track: Option<&str>) -> String {
        self.proxy.stream_url(video_id, itag, track)
    }

    /// Signed proxy URL of a subtitle, see [`proxy::subtitle_id`].
    pub fn subtitle_url(&self, video_id: &str, id: &str) -> String {
        self.proxy.subtitle_url(video_id, id)
    }
}

pub fn app(state: AppState) -> Router {
    let session = axum::middleware::from_fn_with_state(state.clone(), auth::middleware);
    let mut api = api::router().layer(session.clone());
    let mut proxy = proxy::router().layer(session);
    if let Some(rate_limit) = &state.rate_limit {
        api = api.layer(rate_limit.layer());
        proxy = proxy.layer(rate_limit.layer());
    }
    // Media is already compressed, and compressing it would drop range support.
    let compress = DefaultPredicate::new()
        .and(NotForContentType::const_new("video/"))
        .and(NotForContentType::const_new("audio/"));

    Router::new()
        .merge(ops::router())
        .merge(api)
        .merge(proxy)
        .route_layer(axum::middleware::from_fn(telemetry::track_http))
        .fallback(spa::fallback)
        .layer(CompressionLayer::new().compress_when(compress))
        .layer(
            TraceLayer::new_for_http()
                .make_span_with(DefaultMakeSpan::new().level(Level::INFO))
                .on_response(DefaultOnResponse::new().level(Level::INFO)),
        )
        .with_state(state)
}

pub async fn run(config: Config) -> anyhow::Result<()> {
    telemetry::init_tracing(&config.log_filter, config.log_format)?;
    let metrics = telemetry::install_metrics_recorder()?;
    tracing::debug!(?config, "loaded configuration");

    let pool_options = oxistream_db::PoolOptions {
        max_connections: config.database_max_connections,
        ..Default::default()
    };
    let db = oxistream_db::connect(&config.database_url, &pool_options).await?;
    oxistream_db::migrate(&db).await?;
    tracing::info!("database migrations applied");

    let youtube = youtube::YouTube::new(&config.youtube)?;
    let proxy = proxy::Proxy::new(&config.proxy)?;
    let rate_limit = config.rate_limit.as_ref().map(rate_limit::RateLimit::new);
    let auth = auth::Auth::new(
        config.auth.clone(),
        Arc::new(PgUserRepository::new(db.clone())),
        Arc::new(PgSessionRepository::new(db.clone())),
        config.rate_limit.as_ref(),
    );
    auth.spawn_cleanup();
    if let Some(oidc) = auth.oidc() {
        oidc.spawn_discovery();
    }
    if !config.auth.cookie_secure {
        tracing::warn!("session cookies are sent without the Secure flag");
    }
    if let Some(rate_limit) = &rate_limit {
        rate_limit.spawn_cleanup();
    } else {
        tracing::warn!("rate limiting is disabled");
    }
    if let Some(version) = youtube.botguard_version().await {
        tracing::info!(%version, "using rustypipe-botguard");
    } else {
        tracing::warn!("rustypipe-botguard not found, PO tokens are unavailable");
    }

    let listener = TcpListener::bind(config.bind).await?;
    tracing::info!(addr = %listener.local_addr()?, "listening");

    axum::serve(
        listener,
        app(AppState {
            videos: Arc::new(PgVideoRepository::new(db.clone())),
            db: db.clone(),
            metrics,
            youtube,
            proxy,
            rate_limit,
            auth,
        })
        .into_make_service_with_connect_info::<SocketAddr>(),
    )
    .with_graceful_shutdown(shutdown_signal())
    .await?;

    db.close().await;
    tracing::info!("shut down");
    Ok(())
}

async fn shutdown_signal() {
    let ctrl_c = async {
        if let Err(err) = tokio::signal::ctrl_c().await {
            tracing::error!(error = %err, "failed to listen for ctrl-c");
            std::future::pending::<()>().await;
        }
    };

    #[cfg(unix)]
    let terminate = async {
        match tokio::signal::unix::signal(tokio::signal::unix::SignalKind::terminate()) {
            Ok(mut signal) => {
                signal.recv().await;
            }
            Err(err) => {
                tracing::error!(error = %err, "failed to listen for SIGTERM");
                std::future::pending::<()>().await;
            }
        }
    };

    #[cfg(not(unix))]
    let terminate = std::future::pending::<()>();

    tokio::select! {
        () = ctrl_c => {},
        () = terminate => {},
    }
    tracing::info!("shutdown signal received");
}
