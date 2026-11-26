//! Operational endpoints: liveness, readiness and metrics.

use std::time::Duration;

use axum::Router;
use axum::extract::State;
use axum::http::{StatusCode, header};
use axum::response::IntoResponse;
use axum::routing::get;

use crate::AppState;

const READY_TIMEOUT: Duration = Duration::from_secs(2);

pub fn router() -> Router<AppState> {
    Router::new()
        .route("/healthz", get(healthz))
        .route("/readyz", get(readyz))
        .route("/metrics", get(metrics))
}

async fn healthz() -> &'static str {
    "ok"
}

async fn readyz(State(state): State<AppState>) -> (StatusCode, &'static str) {
    if state.auth.oidc().is_some_and(|oidc| !oidc.is_ready()) {
        return (
            StatusCode::SERVICE_UNAVAILABLE,
            "OIDC provider not discovered",
        );
    }
    match tokio::time::timeout(READY_TIMEOUT, oxistream_db::ping(&state.db)).await {
        Ok(Ok(())) => (StatusCode::OK, "ok"),
        Ok(Err(err)) => {
            tracing::warn!(error = ?err, "readiness check failed");
            (StatusCode::SERVICE_UNAVAILABLE, "database unavailable")
        }
        Err(_) => {
            tracing::warn!("readiness check timed out");
            (StatusCode::SERVICE_UNAVAILABLE, "database unavailable")
        }
    }
}

async fn metrics(State(state): State<AppState>) -> impl IntoResponse {
    (
        [(header::CONTENT_TYPE, "text/plain; version=0.0.4")],
        state.metrics.render(),
    )
}
