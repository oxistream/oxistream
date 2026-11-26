//! Needs a Docker daemon; run with `cargo test -- --include-ignored`.

mod common;

use axum::http::StatusCode;
use common::{app_with_pool, get};
use oxistream_db::PoolOptions;
use testcontainers_modules::postgres::Postgres;
use testcontainers_modules::testcontainers::ImageExt;
use testcontainers_modules::testcontainers::runners::AsyncRunner;

#[tokio::test]
#[ignore = "requires Docker"]
async fn readyz_is_ok_against_postgres_18() {
    let container = Postgres::default()
        .with_tag("18-alpine")
        .start()
        .await
        .expect("start postgres container");
    let host = container.get_host().await.unwrap();
    let port = container.get_host_port_ipv4(5432).await.unwrap();
    let url = format!("postgres://postgres:postgres@{host}:{port}/postgres");

    let pool = oxistream_db::connect(&url, &PoolOptions::default())
        .await
        .unwrap();
    oxistream_db::migrate(&pool).await.unwrap();

    let (status, body) = get(app_with_pool(pool), "/readyz").await;
    assert_eq!(status, StatusCode::OK);
    assert_eq!(body, "ok");
}
