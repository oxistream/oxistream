//! Needs `OXISTREAM_TEST_DATABASE_URL` or a Docker daemon; run with `cargo test -- --include-ignored`.

mod common;

use axum::http::StatusCode;
use common::{app_with_pool, get, test_pool};

#[tokio::test]
#[ignore = "requires Docker or OXISTREAM_TEST_DATABASE_URL"]
async fn readyz_is_ok_against_postgres() {
    let db = test_pool().await;

    let (status, body) = get(app_with_pool(db.pool.clone()), "/readyz").await;
    assert_eq!(status, StatusCode::OK);
    assert_eq!(body, "ok");
}
