//! Oxistream HTTP server.

use axum::Router;
use metrics_exporter_prometheus::PrometheusHandle;
use oxistream_db::PgPool;
use tokio::net::TcpListener;
use tower_http::compression::CompressionLayer;
use tower_http::trace::{DefaultMakeSpan, DefaultOnResponse, TraceLayer};
use tracing::Level;

pub mod api;
pub mod config;
mod ops;
mod spa;
pub mod telemetry;

pub use config::Config;

#[derive(Clone)]
pub struct AppState {
    pub db: PgPool,
    pub metrics: PrometheusHandle,
}

pub fn app(state: AppState) -> Router {
    Router::new()
        .merge(ops::router())
        .merge(api::router())
        .route_layer(axum::middleware::from_fn(telemetry::track_http))
        .fallback(spa::fallback)
        .layer(CompressionLayer::new())
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

    let listener = TcpListener::bind(config.bind).await?;
    tracing::info!(addr = %listener.local_addr()?, "listening");

    axum::serve(
        listener,
        app(AppState {
            db: db.clone(),
            metrics,
        }),
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
