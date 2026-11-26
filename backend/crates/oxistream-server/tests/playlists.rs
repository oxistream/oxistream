//! Needs `OXISTREAM_TEST_DATABASE_URL` or a Docker daemon; run with `cargo test -- --include-ignored`.

mod common;

use axum::Router;
use axum::body::Body;
use axum::http::{Method, Request, StatusCode, header};
use common::{isolated_test_pool, state_with_pool, unique_suffix};
use http_body_util::BodyExt;
use oxistream_db::{
    Error, NewPlaylist, NewSession, NewUser, PgPlaylistRepository, PgPool, PgSessionRepository,
    PgUserRepository, PgVideoRepository, PlaylistRepository, PlaylistUpdate, SessionRepository,
    User, UserRepository, VideoRepository, VideoUpsert, Visibility,
};
use oxistream_server::auth::cookie;
use serde_json::{Value, json};
use time::OffsetDateTime;
use tower::ServiceExt;

async fn user(pool: &PgPool) -> User {
    PgUserRepository::new(pool.clone())
        .create(&NewUser {
            username: format!("u_{}", unique_suffix()),
            password_hash: None,
        })
        .await
        .unwrap()
}

async fn cache_videos(pool: &PgPool, ids: &[&str]) {
    let videos = PgVideoRepository::new(pool.clone());
    for id in ids {
        videos
            .upsert(&VideoUpsert {
                id: (*id).to_owned(),
                title: format!("Video {id}"),
                channel_id: "UCaaaaaaaaaaaaaaaaaaaaaa".into(),
                channel_name: "Alpha".into(),
                duration_seconds: Some(213),
                published_at: None,
                thumbnail_url: None,
            })
            .await
            .unwrap();
    }
}

fn new_playlist(id: &str, title: &str) -> NewPlaylist {
    NewPlaylist {
        id: id.to_owned(),
        title: title.to_owned(),
        description: String::new(),
        visibility: Visibility::Private,
    }
}

async fn order(repo: &PgPlaylistRepository, id: &str) -> Vec<String> {
    repo.items(id)
        .await
        .unwrap()
        .into_iter()
        .map(|i| i.video.id)
        .collect()
}

const VIDEOS: [&str; 4] = ["vvvvvvvvvv0", "vvvvvvvvvv1", "vvvvvvvvvv2", "vvvvvvvvvv3"];

#[tokio::test]
#[ignore = "requires Docker or OXISTREAM_TEST_DATABASE_URL"]
async fn items_keep_order_through_add_move_and_remove() {
    let db = isolated_test_pool().await;
    let repo = PgPlaylistRepository::new(db.pool.clone());
    let alice = user(&db.pool).await;
    cache_videos(&db.pool, &VIDEOS).await;
    let id = "pppppppppppppppA";
    let created = repo
        .create(alice.id, &new_playlist(id, "Mix"), 10)
        .await
        .unwrap();
    assert_eq!(created.owner, alice.username);
    assert_eq!(created.item_count, 0);

    for v in VIDEOS {
        assert_eq!(
            repo.add_item(alice.id, id, v, 10).await.unwrap(),
            Some(true)
        );
    }
    assert_eq!(
        repo.add_item(alice.id, id, VIDEOS[1], 10).await.unwrap(),
        Some(false),
        "a video appears once"
    );
    assert_eq!(order(&repo, id).await, VIDEOS);

    assert_eq!(
        repo.move_item(alice.id, id, VIDEOS[0], 2).await.unwrap(),
        Some(true)
    );
    assert_eq!(
        order(&repo, id).await,
        [VIDEOS[1], VIDEOS[2], VIDEOS[0], VIDEOS[3]]
    );
    assert_eq!(
        repo.move_item(alice.id, id, VIDEOS[3], 0).await.unwrap(),
        Some(true)
    );
    assert_eq!(
        order(&repo, id).await,
        [VIDEOS[3], VIDEOS[1], VIDEOS[2], VIDEOS[0]]
    );
    repo.move_item(alice.id, id, VIDEOS[3], 99).await.unwrap();
    assert_eq!(
        order(&repo, id).await,
        [VIDEOS[1], VIDEOS[2], VIDEOS[0], VIDEOS[3]],
        "clamped to the end"
    );
    assert_eq!(
        repo.move_item(alice.id, id, "nnnnnnnnnnn", 0)
            .await
            .unwrap(),
        Some(false)
    );

    assert!(repo.set_last_played(alice.id, id, VIDEOS[2]).await.unwrap());
    assert!(
        !repo
            .set_last_played(alice.id, id, "nnnnnnnnnnn")
            .await
            .unwrap()
    );
    assert_eq!(
        repo.remove_item(alice.id, id, VIDEOS[2]).await.unwrap(),
        Some(true)
    );
    assert_eq!(
        repo.remove_item(alice.id, id, VIDEOS[2]).await.unwrap(),
        Some(false)
    );
    let items = repo.items(id).await.unwrap();
    let positions: Vec<_> = items.iter().map(|i| i.position).collect();
    assert_eq!(positions, [0, 1, 2], "dense after removal");
    let playlist = repo.get(id).await.unwrap().unwrap();
    assert_eq!(playlist.item_count, 3);
    assert_eq!(playlist.first_video_id.as_deref(), Some(VIDEOS[1]));
    assert_eq!(
        playlist.last_video_id, None,
        "removed item is no resume point"
    );
}

#[tokio::test]
#[ignore = "requires Docker or OXISTREAM_TEST_DATABASE_URL"]
async fn owners_only_and_limits() {
    let db = isolated_test_pool().await;
    let repo = PgPlaylistRepository::new(db.pool.clone());
    let alice = user(&db.pool).await;
    let bob = user(&db.pool).await;
    cache_videos(&db.pool, &VIDEOS).await;
    let id = "pppppppppppppppB";
    repo.create(alice.id, &new_playlist(id, "Alice"), 2)
        .await
        .unwrap();
    repo.create(alice.id, &new_playlist("pppppppppppppppC", "Two"), 2)
        .await
        .unwrap();
    assert!(matches!(
        repo.create(alice.id, &new_playlist("pppppppppppppppD", "Three"), 2)
            .await,
        Err(Error::LimitReached)
    ));

    assert_eq!(
        repo.add_item(bob.id, id, VIDEOS[0], 10).await.unwrap(),
        None
    );
    assert_eq!(repo.remove_item(bob.id, id, VIDEOS[0]).await.unwrap(), None);
    assert_eq!(
        repo.move_item(bob.id, id, VIDEOS[0], 0).await.unwrap(),
        None
    );
    assert!(
        repo.update(bob.id, id, &PlaylistUpdate::default())
            .await
            .unwrap()
            .is_none()
    );
    assert!(!repo.delete(bob.id, id).await.unwrap());

    repo.add_item(alice.id, id, VIDEOS[0], 2).await.unwrap();
    repo.add_item(alice.id, id, VIDEOS[1], 2).await.unwrap();
    assert!(matches!(
        repo.add_item(alice.id, id, VIDEOS[2], 2).await,
        Err(Error::LimitReached)
    ));
    assert_eq!(
        repo.add_item(alice.id, id, VIDEOS[1], 2).await.unwrap(),
        Some(false),
        "re-adding an item is fine when full"
    );

    let updated = repo
        .update(
            alice.id,
            id,
            &PlaylistUpdate {
                title: Some("Renamed".into()),
                description: None,
                visibility: Some(Visibility::Unlisted),
            },
        )
        .await
        .unwrap()
        .unwrap();
    assert_eq!(
        (updated.title.as_str(), updated.visibility),
        ("Renamed", Visibility::Unlisted)
    );

    let list = repo.list(alice.id, Some(VIDEOS[0])).await.unwrap();
    assert_eq!(list.len(), 2);
    assert_eq!(list[0].id, id, "most recently changed first");
    assert_eq!(list[0].contains_video, Some(true));
    assert_eq!(list[1].contains_video, Some(false));
    assert_eq!(
        repo.list(alice.id, None).await.unwrap()[0].contains_video,
        None
    );
    assert_eq!(repo.list(bob.id, None).await.unwrap().len(), 0);

    assert!(repo.delete(alice.id, id).await.unwrap());
    assert!(repo.get(id).await.unwrap().is_none());
    assert_eq!(repo.items(id).await.unwrap().len(), 0);
}

struct Api {
    app: Router,
}

impl Api {
    fn new(pool: &PgPool) -> Self {
        Self {
            app: oxistream_server::app(state_with_pool(pool.clone())),
        }
    }

    async fn session(pool: &PgPool) -> String {
        let user = user(pool).await;
        let (token, token_hash) = cookie::new_token();
        PgSessionRepository::new(pool.clone())
            .create(&NewSession {
                token_hash,
                user_id: user.id,
                expires_at: OffsetDateTime::now_utc() + time::Duration::days(1),
                user_agent: None,
                ip: None,
                oidc_sid: None,
                oidc_id_token: None,
            })
            .await
            .unwrap();
        token
    }

    async fn send(
        &self,
        method: Method,
        uri: &str,
        token: Option<&str>,
        body: Option<Value>,
    ) -> (StatusCode, Value) {
        let mut req = Request::builder()
            .method(method)
            .uri(uri)
            .header(header::HOST, "tube.test")
            .header(header::ORIGIN, "http://tube.test");
        if let Some(token) = token {
            req = req.header(header::COOKIE, format!("oxistream_session={token}"));
        }
        let body = match body {
            Some(json) => {
                req = req.header(header::CONTENT_TYPE, "application/json");
                Body::from(json.to_string())
            }
            None => Body::empty(),
        };
        let response = self
            .app
            .clone()
            .oneshot(req.body(body).unwrap())
            .await
            .unwrap();
        let status = response.status();
        let bytes = response.into_body().collect().await.unwrap().to_bytes();
        (
            status,
            serde_json::from_slice(&bytes).unwrap_or(Value::Null),
        )
    }
}

#[tokio::test]
#[ignore = "requires Docker or OXISTREAM_TEST_DATABASE_URL"]
async fn api_requires_login() {
    let db = isolated_test_pool().await;
    let api = Api::new(&db.pool);
    let item = "/api/v0/playlists/pppppppppppppppA/items/vvvvvvvvvv0";
    for (method, uri) in [
        (Method::GET, "/api/v0/playlists"),
        (Method::POST, "/api/v0/playlists"),
        (Method::PATCH, "/api/v0/playlists/pppppppppppppppA"),
        (Method::DELETE, "/api/v0/playlists/pppppppppppppppA"),
        (Method::PUT, item),
        (Method::DELETE, item),
        (Method::PUT, &format!("{item}/position")),
        (
            Method::PUT,
            "/api/v0/playlists/pppppppppppppppA/last-played",
        ),
    ] {
        let (status, json) = api.send(method.clone(), uri, None, None).await;
        assert_eq!(status, StatusCode::UNAUTHORIZED, "{method} {uri}: {json}");
    }
}

#[tokio::test]
#[ignore = "requires Docker or OXISTREAM_TEST_DATABASE_URL"]
#[allow(clippy::too_many_lines)]
async fn api_manages_and_shares_playlists() {
    let db = isolated_test_pool().await;
    cache_videos(&db.pool, &VIDEOS).await;
    let api = Api::new(&db.pool);
    let alice = Api::session(&db.pool).await;
    let bob = Api::session(&db.pool).await;
    let alice = Some(alice.as_str());
    let bob = Some(bob.as_str());

    let (status, json) = api
        .send(
            Method::POST,
            "/api/v0/playlists",
            alice,
            Some(json!({ "title": "  " })),
        )
        .await;
    assert_eq!(status, StatusCode::UNPROCESSABLE_ENTITY);
    assert_eq!(json["fields"]["title"], "required");

    let (status, created) = api
        .send(
            Method::POST,
            "/api/v0/playlists",
            alice,
            Some(json!({ "title": " Road trips ", "description": "Long drives" })),
        )
        .await;
    assert_eq!(status, StatusCode::CREATED, "{created}");
    assert_eq!(created["title"], "Road trips");
    assert_eq!(created["visibility"], "private");
    let id = created["id"].as_str().unwrap().to_owned();
    assert_eq!(id.len(), 16);
    let base = format!("/api/v0/playlists/{id}");

    for v in &VIDEOS[..3] {
        let (status, json) = api
            .send(Method::PUT, &format!("{base}/items/{v}"), alice, None)
            .await;
        assert_eq!(status, StatusCode::CREATED, "{json}");
    }
    let (status, json) = api
        .send(
            Method::PUT,
            &format!("{base}/items/{}", VIDEOS[0]),
            alice,
            None,
        )
        .await;
    assert_eq!(
        (status, json["item_count"].as_u64()),
        (StatusCode::OK, Some(3))
    );
    assert!(
        json["thumbnails"][0]["url"]
            .as_str()
            .unwrap()
            .starts_with("/proxy/image/")
    );

    let (status, _) = api
        .send(
            Method::PUT,
            &format!("{base}/items/{}/position", VIDEOS[2]),
            alice,
            Some(json!({ "index": 0 })),
        )
        .await;
    assert_eq!(status, StatusCode::NO_CONTENT);
    let (status, _) = api
        .send(
            Method::PUT,
            &format!("{base}/last-played"),
            alice,
            Some(json!({ "video_id": VIDEOS[1] })),
        )
        .await;
    assert_eq!(status, StatusCode::NO_CONTENT);

    let (status, details) = api.send(Method::GET, &base, alice, None).await;
    assert_eq!(status, StatusCode::OK, "{details}");
    let ids: Vec<_> = details["items"]
        .as_array()
        .unwrap()
        .iter()
        .map(|i| i["video"]["id"].as_str().unwrap())
        .collect();
    assert_eq!(ids, [VIDEOS[2], VIDEOS[0], VIDEOS[1]]);
    assert_eq!(details["is_owner"], true);
    assert_eq!(details["last_video_id"], VIDEOS[1]);
    assert_eq!(details["items"][0]["video"]["channel"]["name"], "Alpha");

    let (_, list) = api
        .send(
            Method::GET,
            &format!("/api/v0/playlists?video_id={}", VIDEOS[3]),
            alice,
            None,
        )
        .await;
    assert_eq!(list[0]["contains_video"], false);

    // Private: invisible to others, as if it did not exist.
    for token in [None, bob] {
        let (status, _) = api.send(Method::GET, &base, token, None).await;
        assert_eq!(status, StatusCode::NOT_FOUND);
    }
    let (status, _) = api
        .send(
            Method::PUT,
            &format!("{base}/items/{}", VIDEOS[3]),
            bob,
            None,
        )
        .await;
    assert_eq!(status, StatusCode::NOT_FOUND);

    let (status, json) = api
        .send(
            Method::PATCH,
            &base,
            alice,
            Some(json!({ "visibility": "unlisted" })),
        )
        .await;
    assert_eq!(
        (status, json["visibility"].as_str()),
        (StatusCode::OK, Some("unlisted"))
    );
    for token in [None, bob] {
        let (status, shared) = api.send(Method::GET, &base, token, None).await;
        assert_eq!(status, StatusCode::OK);
        assert_eq!(shared["is_owner"], false);
        assert!(
            shared["last_video_id"].is_null(),
            "resume point stays private"
        );
        assert_eq!(shared["items"].as_array().unwrap().len(), 3);
    }
    let (status, _) = api.send(Method::DELETE, &base, bob, None).await;
    assert_eq!(status, StatusCode::NOT_FOUND, "others cannot delete");

    let (status, _) = api
        .send(
            Method::DELETE,
            &format!("{base}/items/{}", VIDEOS[0]),
            alice,
            None,
        )
        .await;
    assert_eq!(status, StatusCode::NO_CONTENT);
    let (status, _) = api.send(Method::DELETE, &base, alice, None).await;
    assert_eq!(status, StatusCode::NO_CONTENT);
    let (status, _) = api.send(Method::GET, &base, alice, None).await;
    assert_eq!(status, StatusCode::NOT_FOUND);
}

#[tokio::test]
#[ignore = "requires Docker or OXISTREAM_TEST_DATABASE_URL"]
async fn api_rejects_bad_input() {
    let db = isolated_test_pool().await;
    let api = Api::new(&db.pool);
    let token = Api::session(&db.pool).await;
    let token = Some(token.as_str());
    let (_, created) = api
        .send(
            Method::POST,
            "/api/v0/playlists",
            token,
            Some(json!({ "title": "x" })),
        )
        .await;
    let base = format!("/api/v0/playlists/{}", created["id"].as_str().unwrap());

    let long = "x".repeat(151);
    for (method, uri, body, expected) in [
        (
            Method::GET,
            "/api/v0/playlists/PLrAXtmErZgOeiKm4sgNOknGvNjby9efdf".to_owned(),
            None,
            StatusCode::NOT_FOUND,
        ),
        (
            Method::GET,
            "/api/v0/playlists?video_id=nope".to_owned(),
            None,
            StatusCode::BAD_REQUEST,
        ),
        (
            Method::PUT,
            format!("{base}/items/nope"),
            None,
            StatusCode::BAD_REQUEST,
        ),
        (
            Method::PATCH,
            base.clone(),
            Some(json!({ "title": long })),
            StatusCode::UNPROCESSABLE_ENTITY,
        ),
        (
            Method::PATCH,
            base.clone(),
            Some(json!({ "visibility": "public" })),
            StatusCode::UNPROCESSABLE_ENTITY,
        ),
        (
            Method::PUT,
            format!("{base}/items/vvvvvvvvvv0/position"),
            Some(json!({ "index": 0 })),
            StatusCode::NOT_FOUND,
        ),
    ] {
        let (status, json) = api.send(method.clone(), &uri, token, body).await;
        assert_eq!(status, expected, "{method} {uri}: {json}");
    }
}
