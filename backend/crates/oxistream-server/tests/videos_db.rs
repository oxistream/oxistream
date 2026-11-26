//! Needs `OXISTREAM_TEST_DATABASE_URL` or a Docker daemon; run with `cargo test -- --include-ignored`.

mod common;

use common::test_pool;
use oxistream_db::{PgVideoRepository, VideoRepository, VideoUpsert};
use time::macros::datetime;

fn video(id: &str) -> VideoUpsert {
    VideoUpsert {
        id: id.to_owned(),
        title: "First title".into(),
        channel_id: "UCuAXFkgsw1L7xaCfnd5JJOw".into(),
        channel_name: "Rick Astley".into(),
        duration_seconds: Some(213),
        published_at: Some(datetime!(2009-10-25 06:57:33 UTC)),
        thumbnail_url: Some("https://i.ytimg.com/vi/x/hq720.jpg".into()),
    }
}

#[tokio::test]
#[ignore = "requires Docker or OXISTREAM_TEST_DATABASE_URL"]
async fn upsert_inserts_and_updates() {
    let db = test_pool().await;
    let repo = PgVideoRepository::new(db.pool.clone());
    let id = "upsertTest1";

    repo.upsert(&video(id)).await.unwrap();
    let first = repo.get(id).await.unwrap().expect("inserted");
    assert_eq!(first.title, "First title");
    assert_eq!(first.duration_seconds, Some(213));
    assert_eq!(first.published_at, Some(datetime!(2009-10-25 06:57:33 UTC)));

    repo.upsert(&VideoUpsert {
        title: "Second title".into(),
        duration_seconds: None,
        published_at: None,
        thumbnail_url: None,
        ..video(id)
    })
    .await
    .unwrap();
    let second = repo.get(id).await.unwrap().expect("still there");
    assert_eq!(second.title, "Second title");
    assert_eq!(
        second.duration_seconds,
        Some(213),
        "unknown fields are kept"
    );
    assert_eq!(second.published_at, first.published_at);
    assert_eq!(second.thumbnail_url, first.thumbnail_url);
    assert!(second.updated_at >= first.updated_at);
}

#[tokio::test]
#[ignore = "requires Docker or OXISTREAM_TEST_DATABASE_URL"]
async fn get_unknown_video_is_none() {
    let db = test_pool().await;
    let repo = PgVideoRepository::new(db.pool.clone());
    assert_eq!(repo.get("doesNotExist").await.unwrap(), None);
}
