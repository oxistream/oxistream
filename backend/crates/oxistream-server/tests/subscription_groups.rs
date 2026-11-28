//! Needs `OXISTREAM_TEST_DATABASE_URL` or a Docker daemon; run with `cargo test -- --include-ignored`.

mod common;

use axum::Router;
use axum::body::Body;
use axum::http::{Method, Request, StatusCode, header};
use common::{app_with_pool, isolated_test_pool, unique_suffix};
use http_body_util::BodyExt;
use oxistream_db::{
    ChannelRepository, ChannelUpdate, ChannelVideoUpsert, Error, FeedFilter, FeedKind,
    GroupChannelOutcome, NewSession, NewSubscriptionGroup, NewUser, PgChannelRepository, PgPool,
    PgSessionRepository, PgSubscriptionGroupRepository, PgSubscriptionRepository, PgUserRepository,
    SessionRepository, SubscriptionGroupRepository, SubscriptionGroupUpdate,
    SubscriptionRepository, User, UserRepository,
};
use oxistream_server::auth::cookie;
use serde_json::{Value, json};
use time::OffsetDateTime;
use time::macros::datetime;
use tower::ServiceExt;

const CHANNEL_A: &str = "UCaaaaaaaaaaaaaaaaaaaaaa";
const CHANNEL_B: &str = "UCbbbbbbbbbbbbbbbbbbbbbb";
const CHANNEL_C: &str = "UCcccccccccccccccccccccc";

async fn user(pool: &PgPool) -> User {
    PgUserRepository::new(pool.clone())
        .create(&NewUser {
            username: format!("u_{}", unique_suffix()),
            password_hash: None,
            admin_if_none: true,
        })
        .await
        .unwrap()
}

fn video(id: &str, day: i64) -> ChannelVideoUpsert {
    ChannelVideoUpsert {
        video_id: id.to_owned(),
        title: format!("Video {id}"),
        duration_seconds: Some(300),
        published_at: Some(datetime!(2026-01-01 00:00 UTC) + time::Duration::days(day)),
        published_text: None,
        thumbnails: Vec::new(),
        view_count: None,
        is_live: false,
        is_upcoming: false,
        is_short: false,
    }
}

/// Channels A (a1, a3), B (b2, b4) and C (c5), each with videos on alternating days.
async fn store_channels(pool: &PgPool) {
    let channels = PgChannelRepository::new(pool.clone());
    for (id, name, videos) in [
        (
            CHANNEL_A,
            "Alpha",
            vec![video("aaaaaaaaaa1", 1), video("aaaaaaaaaa3", 3)],
        ),
        (
            CHANNEL_B,
            "Beta",
            vec![video("bbbbbbbbbb2", 2), video("bbbbbbbbbb4", 4)],
        ),
        (CHANNEL_C, "Gamma", vec![video("cccccccccc5", 5)]),
    ] {
        channels
            .store_fetch(
                &ChannelUpdate {
                    id: id.to_owned(),
                    name: name.to_owned(),
                    handle: None,
                    avatar: Vec::new(),
                    subscriber_count: None,
                    verified: false,
                },
                &videos,
            )
            .await
            .unwrap();
    }
}

async fn subscribed_user(pool: &PgPool) -> User {
    let subs = PgSubscriptionRepository::new(pool.clone());
    let alice = user(pool).await;
    for id in [CHANNEL_A, CHANNEL_B, CHANNEL_C] {
        subs.subscribe(alice.id, id).await.unwrap();
    }
    alice
}

fn new_group(name: &str) -> NewSubscriptionGroup {
    NewSubscriptionGroup {
        name: name.to_owned(),
        show_on_home: false,
        channel_ids: Vec::new(),
    }
}

async fn names(groups: &PgSubscriptionGroupRepository, user: &User) -> Vec<String> {
    groups
        .list(user.id)
        .await
        .unwrap()
        .into_iter()
        .map(|g| g.name)
        .collect()
}

#[tokio::test]
#[ignore = "requires Docker or OXISTREAM_TEST_DATABASE_URL"]
async fn groups_keep_order_names_and_limit() {
    let db = isolated_test_pool().await;
    let groups = PgSubscriptionGroupRepository::new(db.pool.clone());
    let alice = user(&db.pool).await;
    let bob = user(&db.pool).await;

    let tech = groups
        .create(alice.id, &new_group("Tech"), 3)
        .await
        .unwrap();
    let music = groups
        .create(alice.id, &new_group("Music"), 3)
        .await
        .unwrap();
    let news = groups
        .create(alice.id, &new_group("News"), 3)
        .await
        .unwrap();
    assert!(matches!(
        groups.create(alice.id, &new_group("More"), 3).await,
        Err(Error::LimitReached)
    ));
    assert_eq!(names(&groups, &alice).await, ["Tech", "Music", "News"]);

    // Unique per user, ignoring case.
    assert!(
        groups.create(bob.id, &new_group("tech"), 3).await.is_ok(),
        "unique per user"
    );
    let rename = |name: &str| SubscriptionGroupUpdate {
        name: Some(name.to_owned()),
        ..SubscriptionGroupUpdate::default()
    };
    assert!(matches!(
        groups.update(alice.id, music.id, &rename("TECH")).await,
        Err(Error::NameTaken)
    ));
    let renamed = groups
        .update(alice.id, music.id, &rename("Songs"))
        .await
        .unwrap()
        .unwrap();
    assert_eq!(renamed.name, "Songs");
    assert!(!renamed.show_on_home);
    let shown = groups
        .update(
            alice.id,
            music.id,
            &SubscriptionGroupUpdate {
                show_on_home: Some(true),
                ..SubscriptionGroupUpdate::default()
            },
        )
        .await
        .unwrap()
        .unwrap();
    assert!(shown.show_on_home);
    assert_eq!(shown.name, "Songs");
    assert_eq!(
        groups
            .update(bob.id, music.id, &rename("Mine"))
            .await
            .unwrap(),
        None,
        "foreign groups do not exist"
    );

    assert!(groups.move_to(alice.id, news.id, 0).await.unwrap());
    assert_eq!(names(&groups, &alice).await, ["News", "Tech", "Songs"]);
    assert!(groups.move_to(alice.id, news.id, 99).await.unwrap());
    assert_eq!(names(&groups, &alice).await, ["Tech", "Songs", "News"]);
    assert!(!groups.move_to(bob.id, news.id, 0).await.unwrap());

    assert!(!groups.delete(bob.id, tech.id).await.unwrap());
    assert!(groups.delete(alice.id, tech.id).await.unwrap());
    assert!(!groups.delete(alice.id, tech.id).await.unwrap());
    assert_eq!(names(&groups, &alice).await, ["Songs", "News"]);
    // Positions stay contiguous, so appending and moving keep working.
    groups
        .create(alice.id, &new_group("Tech"), 3)
        .await
        .unwrap();
    assert!(groups.move_to(alice.id, news.id, 0).await.unwrap());
    assert_eq!(names(&groups, &alice).await, ["News", "Songs", "Tech"]);
}

#[tokio::test]
#[ignore = "requires Docker or OXISTREAM_TEST_DATABASE_URL"]
#[allow(clippy::too_many_lines)]
async fn group_channels_follow_subscriptions() {
    let db = isolated_test_pool().await;
    store_channels(&db.pool).await;
    let groups = PgSubscriptionGroupRepository::new(db.pool.clone());
    let subs = PgSubscriptionRepository::new(db.pool.clone());
    let alice = subscribed_user(&db.pool).await;
    let bob = user(&db.pool).await;

    let created = groups
        .create(
            alice.id,
            &NewSubscriptionGroup {
                name: "Tech".into(),
                show_on_home: true,
                channel_ids: vec![
                    CHANNEL_B.into(),
                    CHANNEL_A.into(),
                    "UCzzzzzzzzzzzzzzzzzzzzzz".into(),
                ],
            },
            50,
        )
        .await
        .unwrap();
    assert!(created.show_on_home);
    assert_eq!(
        created.channel_ids,
        [CHANNEL_A, CHANNEL_B],
        "unsubscribed left out"
    );

    // A channel can be in several groups.
    let other = groups
        .create(alice.id, &new_group("Other"), 50)
        .await
        .unwrap();
    assert_eq!(
        groups
            .add_channel(alice.id, other.id, CHANNEL_A)
            .await
            .unwrap(),
        GroupChannelOutcome::Done
    );
    assert_eq!(
        groups
            .add_channel(alice.id, other.id, CHANNEL_A)
            .await
            .unwrap(),
        GroupChannelOutcome::Done,
        "idempotent"
    );
    assert_eq!(
        groups
            .add_channel(bob.id, other.id, CHANNEL_A)
            .await
            .unwrap(),
        GroupChannelOutcome::GroupNotFound
    );
    let bobs = groups.create(bob.id, &new_group("Bob"), 50).await.unwrap();
    assert_eq!(
        groups
            .add_channel(bob.id, bobs.id, CHANNEL_A)
            .await
            .unwrap(),
        GroupChannelOutcome::NotSubscribed
    );

    subs.unsubscribe(alice.id, CHANNEL_A).await.unwrap();
    let list = groups.list(alice.id).await.unwrap();
    assert_eq!(list[0].channel_ids, [CHANNEL_B]);
    assert_eq!(list[1].channel_ids.len(), 0);
    subs.subscribe(alice.id, CHANNEL_A).await.unwrap();
    assert_eq!(
        groups
            .get(alice.id, created.id)
            .await
            .unwrap()
            .unwrap()
            .channel_ids,
        [CHANNEL_B],
        "resubscribing does not restore groups"
    );

    assert!(
        groups
            .remove_channel(alice.id, created.id, CHANNEL_B)
            .await
            .unwrap()
    );
    assert!(
        groups
            .remove_channel(alice.id, created.id, CHANNEL_B)
            .await
            .unwrap()
    );
    assert!(
        !groups
            .remove_channel(bob.id, created.id, CHANNEL_B)
            .await
            .unwrap()
    );
    assert_eq!(
        groups
            .get(alice.id, created.id)
            .await
            .unwrap()
            .unwrap()
            .channel_ids
            .len(),
        0
    );

    // Deleting a group keeps the subscriptions.
    groups
        .add_channel(alice.id, created.id, CHANNEL_C)
        .await
        .unwrap();
    assert!(groups.delete(alice.id, created.id).await.unwrap());
    assert!(subs.is_subscribed(alice.id, CHANNEL_C).await.unwrap());
}

#[tokio::test]
#[ignore = "requires Docker or OXISTREAM_TEST_DATABASE_URL"]
async fn feed_filters_by_group_across_pages() {
    let db = isolated_test_pool().await;
    store_channels(&db.pool).await;
    let groups = PgSubscriptionGroupRepository::new(db.pool.clone());
    let subs = PgSubscriptionRepository::new(db.pool.clone());
    let alice = subscribed_user(&db.pool).await;
    let group = groups
        .create(
            alice.id,
            &NewSubscriptionGroup {
                name: "AC".into(),
                show_on_home: false,
                channel_ids: vec![CHANNEL_A.into(), CHANNEL_C.into()],
            },
            50,
        )
        .await
        .unwrap();
    let filter = FeedFilter {
        kind: FeedKind::Videos,
        group_id: Some(group.id),
        ..FeedFilter::default()
    };

    let first = subs.feed(alice.id, &filter, None, 2).await.unwrap();
    let ids: Vec<_> = first.iter().map(|v| v.video_id.as_str()).collect();
    assert_eq!(ids, ["cccccccccc5", "aaaaaaaaaa3"]);
    let cursor = first.last().unwrap().cursor();
    let second = subs
        .feed(alice.id, &filter, Some(&cursor), 2)
        .await
        .unwrap();
    let ids: Vec<_> = second.iter().map(|v| v.video_id.as_str()).collect();
    assert_eq!(ids, ["aaaaaaaaaa1"]);

    let only_c = FeedFilter {
        channel_id: Some(CHANNEL_C.into()),
        ..filter.clone()
    };
    assert_eq!(
        subs.feed(alice.id, &only_c, None, 10).await.unwrap().len(),
        1
    );
    let only_b = FeedFilter {
        channel_id: Some(CHANNEL_B.into()),
        ..filter.clone()
    };
    assert_eq!(
        subs.feed(alice.id, &only_b, None, 10).await.unwrap().len(),
        0
    );

    // Another user's group never matches.
    let bob = subscribed_user(&db.pool).await;
    assert_eq!(subs.feed(bob.id, &filter, None, 10).await.unwrap().len(), 0);
}

struct Api {
    app: Router,
    token: String,
}

impl Api {
    async fn new(pool: &PgPool, user: &User) -> Self {
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
        Self {
            app: app_with_pool(pool.clone()),
            token,
        }
    }

    async fn send(
        &self,
        method: Method,
        uri: &str,
        signed_in: bool,
        body: Option<Value>,
    ) -> (StatusCode, Value) {
        let mut req = Request::builder()
            .method(method)
            .uri(uri)
            .header(header::HOST, "tube.test")
            .header(header::ORIGIN, "http://tube.test");
        if signed_in {
            req = req.header(header::COOKIE, format!("oxistream_session={}", self.token));
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

    async fn feed(&self, query: &str) -> (StatusCode, Vec<String>) {
        let (status, page) = self
            .send(Method::GET, &format!("/api/v0/feed?{query}"), true, None)
            .await;
        let ids = page["items"]
            .as_array()
            .map(|items| {
                items
                    .iter()
                    .map(|v| v["id"].as_str().unwrap().to_owned())
                    .collect()
            })
            .unwrap_or_default();
        (status, ids)
    }
}

#[tokio::test]
#[ignore = "requires Docker or OXISTREAM_TEST_DATABASE_URL"]
#[allow(clippy::too_many_lines)]
async fn api_manages_groups_and_filters_feed() {
    let db = isolated_test_pool().await;
    store_channels(&db.pool).await;
    let alice = subscribed_user(&db.pool).await;
    let api = Api::new(&db.pool, &alice).await;

    let (status, _) = api
        .send(Method::GET, "/api/v0/subscription-groups", false, None)
        .await;
    assert_eq!(status, StatusCode::UNAUTHORIZED);

    let (status, tech) = api
        .send(
            Method::POST,
            "/api/v0/subscription-groups",
            true,
            Some(json!({ "name": "  Tech  ", "channel_ids": [CHANNEL_B] })),
        )
        .await;
    assert_eq!(status, StatusCode::CREATED, "{tech}");
    assert_eq!(tech["name"], "Tech");
    assert_eq!(tech["show_on_home"], false);
    assert_eq!(tech["channel_ids"], json!([CHANNEL_B]));
    let id = tech["id"].as_str().unwrap().to_owned();

    for (body, field, error) in [
        (json!({ "name": "tech" }), "name", "taken"),
        (json!({ "name": " " }), "name", "required"),
        (json!({ "name": "x".repeat(51) }), "name", "too_long"),
    ] {
        let (status, json) = api
            .send(
                Method::POST,
                "/api/v0/subscription-groups",
                true,
                Some(body),
            )
            .await;
        assert_eq!(status, StatusCode::UNPROCESSABLE_ENTITY, "{json}");
        assert_eq!(json["fields"][field], error);
    }
    let (status, _) = api
        .send(
            Method::POST,
            "/api/v0/subscription-groups",
            true,
            Some(json!({ "name": "Bad", "channel_ids": ["nope"] })),
        )
        .await;
    assert_eq!(status, StatusCode::BAD_REQUEST);

    let (status, json) = api
        .send(
            Method::PUT,
            &format!("/api/v0/subscription-groups/{id}/channels/{CHANNEL_C}"),
            true,
            None,
        )
        .await;
    assert_eq!(status, StatusCode::NO_CONTENT, "{json}");
    assert_eq!(
        api.feed(&format!("kind=videos&group={id}")).await,
        (
            StatusCode::OK,
            vec![
                "cccccccccc5".into(),
                "bbbbbbbbbb4".into(),
                "bbbbbbbbbb2".into()
            ]
        )
    );
    assert_eq!(
        api.feed(&format!("kind=videos&group={id}&limit=1")).await.1,
        ["cccccccccc5"]
    );
    let (status, _) = api
        .send(
            Method::DELETE,
            &format!("/api/v0/subscription-groups/{id}/channels/{CHANNEL_C}"),
            true,
            None,
        )
        .await;
    assert_eq!(status, StatusCode::NO_CONTENT);
    assert_eq!(api.feed(&format!("group={id}")).await.1.len(), 2);

    let (status, news) = api
        .send(
            Method::POST,
            "/api/v0/subscription-groups",
            true,
            Some(json!({ "name": "News", "show_on_home": true })),
        )
        .await;
    assert_eq!(status, StatusCode::CREATED);
    let news_id = news["id"].as_str().unwrap();
    let (status, _) = api
        .send(
            Method::PUT,
            &format!("/api/v0/subscription-groups/{news_id}/position"),
            true,
            Some(json!({ "index": 0 })),
        )
        .await;
    assert_eq!(status, StatusCode::NO_CONTENT);
    let (status, renamed) = api
        .send(
            Method::PATCH,
            &format!("/api/v0/subscription-groups/{id}"),
            true,
            Some(json!({ "name": "Gadgets", "show_on_home": true })),
        )
        .await;
    assert_eq!(status, StatusCode::OK, "{renamed}");
    assert_eq!(renamed["name"], "Gadgets");
    assert_eq!(renamed["show_on_home"], true);
    let (_, list) = api
        .send(Method::GET, "/api/v0/subscription-groups", true, None)
        .await;
    let listed: Vec<_> = list
        .as_array()
        .unwrap()
        .iter()
        .map(|g| g["name"].as_str().unwrap())
        .collect();
    assert_eq!(listed, ["News", "Gadgets"]);

    // Other users' groups do not exist for them.
    let bob = subscribed_user(&db.pool).await;
    let bob_api = Api::new(&db.pool, &bob).await;
    for (method, uri) in [
        (Method::GET, format!("/api/v0/feed?group={id}")),
        (Method::GET, "/api/v0/feed?group=nope".to_owned()),
        (Method::PATCH, format!("/api/v0/subscription-groups/{id}")),
        (Method::DELETE, format!("/api/v0/subscription-groups/{id}")),
        (
            Method::PUT,
            format!("/api/v0/subscription-groups/{id}/channels/{CHANNEL_A}"),
        ),
    ] {
        let (status, json) = bob_api
            .send(method.clone(), &uri, true, Some(json!({})))
            .await;
        assert_eq!(status, StatusCode::NOT_FOUND, "{method} {uri}: {json}");
    }
    let (status, _) = api
        .send(
            Method::PUT,
            &format!("/api/v0/subscription-groups/{id}/channels/nope"),
            true,
            None,
        )
        .await;
    assert_eq!(status, StatusCode::BAD_REQUEST);

    let (status, _) = api
        .send(
            Method::DELETE,
            &format!("/api/v0/subscription-groups/{id}"),
            true,
            None,
        )
        .await;
    assert_eq!(status, StatusCode::NO_CONTENT);
    assert_eq!(
        api.feed(&format!("group={id}")).await.0,
        StatusCode::NOT_FOUND
    );
}
