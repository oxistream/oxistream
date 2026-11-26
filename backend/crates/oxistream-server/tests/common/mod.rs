#![allow(dead_code)]

use std::time::Duration;

use axum::Router;
use axum::body::Body;
use axum::http::{Request, StatusCode};
use http_body_util::BodyExt;
use oxistream_db::{PgPool, PoolOptions};
use oxistream_server::{AppState, telemetry};
use tower::ServiceExt;

pub fn app_with_pool(db: PgPool) -> Router {
    oxistream_server::app(AppState {
        db,
        metrics: telemetry::detached_metrics_handle(),
    })
}

/// An app whose pool points at a closed port, for routes that must not need the database.
pub fn app_without_db() -> Router {
    let options = PoolOptions {
        max_connections: 1,
        acquire_timeout: Duration::from_millis(500),
    };
    let pool = oxistream_db::connect_lazy("postgres://oxistream@127.0.0.1:1/oxistream", &options)
        .expect("valid database url");
    app_with_pool(pool)
}

pub async fn get(app: Router, uri: &str) -> (StatusCode, String) {
    let response = app
        .oneshot(Request::get(uri).body(Body::empty()).unwrap())
        .await
        .unwrap();
    let status = response.status();
    let body = response.into_body().collect().await.unwrap().to_bytes();
    (status, String::from_utf8(body.to_vec()).unwrap())
}
