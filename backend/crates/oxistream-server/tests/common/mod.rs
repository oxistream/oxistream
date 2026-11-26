#![allow(dead_code)]

use std::time::Duration;

use axum::Router;
use axum::body::Body;
use axum::http::{Request, StatusCode};
use http_body_util::BodyExt;
use oxistream_db::{PgPool, PoolOptions};
use oxistream_server::{AppState, telemetry};
use testcontainers_modules::postgres::Postgres;
use testcontainers_modules::testcontainers::runners::AsyncRunner;
use testcontainers_modules::testcontainers::{ContainerAsync, ImageExt};
use tower::ServiceExt;

pub const TEST_DATABASE_URL_ENV: &str = "OXISTREAM_TEST_DATABASE_URL";

/// A migrated database pool. Holds the container, if one was started, so it outlives the test.
pub struct TestDb {
    pub pool: PgPool,
    _container: Option<ContainerAsync<Postgres>>,
}

/// Uses `OXISTREAM_TEST_DATABASE_URL` when set, else starts a Postgres 18 container.
pub async fn test_pool() -> TestDb {
    let (url, container) = if let Ok(url) = std::env::var(TEST_DATABASE_URL_ENV) {
        (url, None)
    } else {
        let container = Postgres::default()
            .with_tag("18-alpine")
            .start()
            .await
            .expect("start postgres container");
        let host = container.get_host().await.unwrap();
        let port = container.get_host_port_ipv4(5432).await.unwrap();
        let url = format!("postgres://postgres:postgres@{host}:{port}/postgres");
        (url, Some(container))
    };

    let pool = oxistream_db::connect(&url, &PoolOptions::default())
        .await
        .expect("connect to test database");
    oxistream_db::migrate(&pool)
        .await
        .expect("migrate test database");
    TestDb {
        pool,
        _container: container,
    }
}

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
