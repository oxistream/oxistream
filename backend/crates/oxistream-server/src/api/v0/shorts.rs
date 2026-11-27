use std::collections::{HashSet, VecDeque};

use axum::Json;
use axum::extract::{Path, Query, State};
use oxistream_db::FeedFilter;
use rustypipe::model::paginator::Paginator;
use rustypipe::model::{ShortDetails as RpShortDetails, ShortItem};
use rustypipe::param::{Language, TrendingCategory};
use rustypipe::validate;
use serde::{Deserialize, Serialize};
use utoipa::{IntoParams, ToSchema};
use utoipa_axum::router::OpenApiRouter;
use utoipa_axum::routes;
use uuid::Uuid;

use super::blocks::{BlockedChannels, ChannelItem};
use crate::AppState;
use crate::api::dto::{ContentDisclosure, Thumbnail, thumbnails};
use crate::api::error::{ApiError, ErrorBody};
use crate::api::lang::ContentLanguage;
use crate::auth::OptionalUser;
use crate::youtube::{self, YouTubeError};

/// Liked videos up to this long count as shorts when picking seeds.
const SHORT_MAX_SECONDS: i32 = 180;
const LIKED_SEEDS: usize = 4;
const SUBSCRIPTION_SEEDS: usize = 4;
const TRENDING_SEEDS: usize = 3;
const PAGE_CAP: usize = 30;
/// Seeds tried per page before giving up when they fail upstream.
const SEED_ATTEMPTS: usize = 3;
const MAX_SEEDS: usize = 16;
const MAX_UPSTREAM_TOKEN: usize = 8192;

pub fn router() -> OpenApiRouter<AppState> {
    OpenApiRouter::new()
        .routes(routes!(discover))
        .routes(routes!(short))
}

/// A short to discover. YouTube sends no title or channel name with it; load them with
/// `GET /videos/{id}` when it is shown.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, ToSchema)]
pub struct DiscoveredShort {
    pub id: String,
    pub channel_id: Option<String>,
    pub thumbnails: Vec<Thumbnail>,
}

impl ChannelItem for DiscoveredShort {
    fn channel_id(&self) -> Option<&str> {
        self.channel_id.as_deref()
    }
}

#[derive(Debug, Default, Serialize, ToSchema)]
pub struct ShortsPage {
    pub items: Vec<DiscoveredShort>,
    pub continuation: Option<String>,
}

/// A full-length video a short links to.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, ToSchema)]
pub struct LinkedVideo {
    pub id: String,
    pub title: String,
}

/// What the shorts player shows over a short.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, ToSchema)]
pub struct ShortDetails {
    pub id: String,
    pub title: Option<String>,
    /// The full video the uploader linked to the short.
    pub linked_video: Option<LinkedVideo>,
    /// YouTube's "How this was made" labels, e.g. altered or synthetic content.
    pub content_disclosures: Vec<ContentDisclosure>,
}

impl From<RpShortDetails> for ShortDetails {
    fn from(details: RpShortDetails) -> Self {
        Self {
            id: details.id,
            title: details.name,
            linked_video: details.linked_video.map(|v| LinkedVideo {
                id: v.id,
                title: v.name,
            }),
            content_disclosures: ContentDisclosure::from_rp(details.content_disclosures),
        }
    }
}

#[derive(Debug, Deserialize, IntoParams)]
#[into_params(parameter_in = Query)]
struct DiscoverParams {
    /// Token from a previous page.
    continuation: Option<String>,
}

/// Where the next page comes from: seeds still to start from and the sequence to continue.
#[derive(Debug, Default, PartialEq, Eq)]
struct DiscoverState {
    seeds: VecDeque<String>,
    upstream: Option<String>,
}

impl DiscoverState {
    fn encode(&self) -> Option<String> {
        if self.seeds.is_empty() && self.upstream.is_none() {
            return None;
        }
        let seeds = self.seeds.iter().map(String::as_str).collect::<Vec<_>>();
        Some(format!(
            "{}.{}",
            seeds.join(","),
            self.upstream.as_deref().unwrap_or_default()
        ))
    }

    fn decode(token: &str) -> Option<Self> {
        let (seeds, upstream) = token.split_once('.')?;
        let seeds = seeds
            .split(',')
            .filter(|s| !s.is_empty())
            .map(|s| validate::video_id(s).ok().map(|()| s.to_owned()))
            .collect::<Option<VecDeque<_>>>()?;
        if seeds.len() > MAX_SEEDS
            || upstream.len() > MAX_UPSTREAM_TOKEN
            || !upstream
                .bytes()
                .all(|b| b.is_ascii_alphanumeric() || matches!(b, b'-' | b'_' | b'%' | b'='))
        {
            return None;
        }
        Some(Self {
            seeds,
            upstream: Some(upstream.to_owned()).filter(|u| !u.is_empty()),
        })
    }
}

/// Endless shorts to swipe through, like YouTube's shorts player. Each page continues the
/// current sequence and starts a new one from the next seed, interleaved. Seeds are the
/// signed-in user's liked shorts and the latest shorts of their subscriptions, plus trending
/// videos; anonymous users get trending seeds only. Nothing about the user is sent to YouTube
/// but the seed video IDs. Shorts of blocked channels are left out, so a page may be empty
/// while more follow.
#[utoipa::path(
    get,
    path = "/shorts/discover",
    tag = "shorts",
    params(
        DiscoverParams,
        ("Accept-Language" = Option<String>, Header, description = "Content language"),
    ),
    responses(
        (status = OK, body = ShortsPage),
        (status = BAD_REQUEST, description = "Invalid continuation", body = ErrorBody),
        (status = BAD_GATEWAY, description = "YouTube request failed", body = ErrorBody),
        (status = SERVICE_UNAVAILABLE, description = "Rate limited by YouTube", body = ErrorBody),
    )
)]
async fn discover(
    State(state): State<AppState>,
    OptionalUser(user): OptionalUser,
    Query(params): Query<DiscoverParams>,
    ContentLanguage(lang): ContentLanguage,
    blocked: BlockedChannels,
) -> Result<Json<ShortsPage>, ApiError> {
    let mut current = match params.continuation.as_deref().map(str::trim) {
        None | Some("") => DiscoverState {
            seeds: initial_seeds(&state, user.map(|u| u.id), lang).await?,
            upstream: None,
        },
        Some(token) => DiscoverState::decode(token)
            .ok_or_else(|| ApiError::bad_request("invalid continuation"))?,
    };

    let upstream = current.upstream.take();
    let continued = async {
        match &upstream {
            Some(token) => Some(state.youtube.reel_sequence_continuation(token, lang).await),
            None => None,
        }
    };
    let started = start_sequence(&state, &mut current.seeds, lang);
    let (continued, started) = tokio::join!(continued, started);

    let mut pages = Vec::new();
    let mut error = None;
    for result in [started, continued].into_iter().flatten() {
        match result {
            Ok(page) => pages.push(page),
            Err(err) => {
                tracing::warn!(error = %err, "shorts sequence failed");
                error.get_or_insert(err);
            }
        }
    }
    if pages.is_empty()
        && let Some(err) = error
    {
        return Err(err.into());
    }

    // The newer sequence goes on; the older one has had its turn.
    current.upstream = pages.iter().find_map(|p| p.ctoken.clone());
    let img = |url: &str| state.image_url(url);
    let mut items = interleave(pages)
        .into_iter()
        .map(|s| DiscoveredShort {
            thumbnails: thumbnails(&s.thumbnail, &img),
            id: s.id,
            channel_id: s.channel_id,
        })
        .collect::<Vec<_>>();
    blocked.retain(&mut items);
    Ok(Json(ShortsPage {
        items,
        continuation: current.encode(),
    }))
}

/// The sequence of the next seed that works, if any is left.
async fn start_sequence(
    state: &AppState,
    seeds: &mut VecDeque<String>,
    lang: Language,
) -> Option<youtube::Result<Paginator<ShortItem>>> {
    let mut last = None;
    for _ in 0..SEED_ATTEMPTS {
        let seed = seeds.pop_front()?;
        match state.youtube.reel_sequence(&seed, lang).await {
            Ok(page) if !page.items.is_empty() => return Some(Ok(page)),
            Ok(_) => {}
            Err(YouTubeError::RateLimited(msg)) => {
                return Some(Err(YouTubeError::RateLimited(msg)));
            }
            Err(err) => {
                tracing::debug!(seed, error = %err, "shorts seed failed");
                last = Some(err);
            }
        }
    }
    last.map(Err)
}

/// Takes items from each page in turn, without duplicates, up to [`PAGE_CAP`].
fn interleave(pages: Vec<Paginator<ShortItem>>) -> Vec<ShortItem> {
    let mut iters = pages
        .into_iter()
        .map(|p| p.items.into_iter())
        .collect::<Vec<_>>();
    let mut seen = HashSet::new();
    let mut out = Vec::new();
    loop {
        let mut any = false;
        for iter in &mut iters {
            if let Some(item) = iter.next() {
                any = true;
                if seen.insert(item.id.clone()) {
                    out.push(item);
                }
            }
        }
        if !any || out.len() >= PAGE_CAP {
            out.truncate(PAGE_CAP);
            return out;
        }
    }
}

async fn initial_seeds(
    state: &AppState,
    user: Option<Uuid>,
    lang: Language,
) -> Result<VecDeque<String>, ApiError> {
    let mut groups = Vec::new();
    if let Some(user) = user {
        let liked = state.likes.list(user, None, 50).await?;
        let mut liked = liked
            .into_iter()
            .filter(|l| {
                l.video
                    .duration_seconds
                    .is_none_or(|d| d <= SHORT_MAX_SECONDS)
            })
            .map(|l| l.video.id)
            .take(LIKED_SEEDS * 3)
            .collect::<Vec<_>>();
        shuffle(&mut liked);
        groups.push(liked.into_iter().take(LIKED_SEEDS).collect::<Vec<_>>());

        let feed = state
            .subscriptions
            .feed(user, FeedFilter::Shorts, None, 30)
            .await?;
        let mut subscribed = feed.into_iter().map(|v| v.video_id).collect::<Vec<_>>();
        shuffle(&mut subscribed);
        groups.push(subscribed.into_iter().take(SUBSCRIPTION_SEEDS).collect());
    }
    match state
        .youtube
        .trending(TrendingCategory::Podcasts, lang)
        .await
    {
        Ok(videos) => {
            let mut trending = videos.iter().map(|v| v.id.clone()).collect::<Vec<_>>();
            shuffle(&mut trending);
            groups.push(trending.into_iter().take(TRENDING_SEEDS).collect());
        }
        // Personal seeds are enough to go on with.
        Err(err) if groups.iter().any(|g| !g.is_empty()) => {
            tracing::warn!(error = %err, "trending seeds unavailable");
        }
        Err(err) => return Err(err.into()),
    }
    Ok(mix(groups))
}

/// Round robin over the seed groups, without duplicates.
fn mix(groups: Vec<Vec<String>>) -> VecDeque<String> {
    let mut iters = groups
        .into_iter()
        .map(IntoIterator::into_iter)
        .collect::<Vec<_>>();
    let mut seen = HashSet::new();
    let mut out = VecDeque::new();
    loop {
        let mut any = false;
        for iter in &mut iters {
            if let Some(id) = iter.next() {
                any = true;
                if seen.insert(id.clone()) {
                    out.push_back(id);
                }
            }
        }
        if !any {
            out.truncate(MAX_SEEDS);
            return out;
        }
    }
}

fn shuffle<T>(items: &mut [T]) {
    for i in (1..items.len()).rev() {
        let mut bytes = [0u8; size_of::<usize>()];
        getrandom::fill(&mut bytes).expect("system random number generator");
        let j = usize::from_le_bytes(bytes) % (i + 1);
        items.swap(i, j);
    }
}

/// The shorts player overlay of a short: its title and the full video it links to.
#[utoipa::path(
    get,
    path = "/shorts/{id}",
    tag = "shorts",
    params(
        ("id" = String, Path, description = "YouTube video ID of the short"),
        ("Accept-Language" = Option<String>, Header, description = "Content language"),
    ),
    responses(
        (status = OK, body = ShortDetails),
        (status = BAD_REQUEST, description = "Invalid video ID", body = ErrorBody),
        (status = NOT_FOUND, description = "Not found or not a short", body = ErrorBody),
        (status = BAD_GATEWAY, description = "YouTube request failed", body = ErrorBody),
        (status = SERVICE_UNAVAILABLE, description = "Rate limited by YouTube", body = ErrorBody),
    )
)]
async fn short(
    State(state): State<AppState>,
    Path(id): Path<String>,
    ContentLanguage(lang): ContentLanguage,
) -> Result<Json<ShortDetails>, ApiError> {
    let details = state.youtube.short_details(&id, lang).await?;
    Ok(Json(details.into()))
}

#[cfg(test)]
mod tests {
    use super::*;

    fn ids(items: &[&str]) -> VecDeque<String> {
        items.iter().map(|&s| s.to_owned()).collect()
    }

    #[test]
    fn state_round_trips() {
        let state = DiscoverState {
            seeds: ids(&["dQw4w9WgXcQ", "sHpEqeSByPI"]),
            upstream: Some("Cgs2a1NDVnFa-_ZGhaMA%3D%3D".into()),
        };
        let token = state.encode().unwrap();
        assert_eq!(DiscoverState::decode(&token), Some(state));

        let seeds_only = DiscoverState {
            seeds: ids(&["dQw4w9WgXcQ"]),
            upstream: None,
        };
        assert_eq!(
            DiscoverState::decode(&seeds_only.encode().unwrap()),
            Some(seeds_only)
        );
        assert_eq!(DiscoverState::default().encode(), None);

        // Query strings arrive decoded, so URL-encoded padding turns into `=`.
        let decoded = DiscoverState::decode("dQw4w9WgXcQ.Cgs2a1NDVnFa==").unwrap();
        assert_eq!(decoded.upstream.as_deref(), Some("Cgs2a1NDVnFa=="));
    }

    #[test]
    fn rejects_malformed_state() {
        for token in [
            "",
            "nodot",
            "bad-id!.abc",
            "dQw4w9WgXcQ.a b",
            "dQw4w9WgXcQ.a\"b",
        ] {
            assert_eq!(DiscoverState::decode(token), None, "{token}");
        }
    }

    #[test]
    fn mixes_groups_round_robin() {
        let groups = vec![
            vec!["a".into(), "b".into(), "c".into()],
            vec!["x".into(), "a".into()],
            vec![],
        ];
        assert_eq!(mix(groups), ids(&["a", "x", "b", "c"]));
    }

    fn short(id: &str) -> ShortItem {
        serde_json::from_value(serde_json::json!({ "id": id, "channel_id": null, "thumbnail": [] }))
            .unwrap()
    }

    fn page(items: &[&str]) -> Paginator<ShortItem> {
        let mut page: Paginator<ShortItem> = Paginator::default();
        page.items = items.iter().map(|id| short(id)).collect();
        page
    }

    #[test]
    fn interleaves_pages_without_duplicates() {
        let out = interleave(vec![page(&["a", "b", "c"]), page(&["x", "b", "y"])]);
        let ids = out.iter().map(|s| s.id.as_str()).collect::<Vec<_>>();
        assert_eq!(ids, ["a", "x", "b", "c", "y"]);
    }

    #[test]
    fn caps_a_page() {
        let many = (0..50).map(|i| format!("v{i}")).collect::<Vec<_>>();
        let refs = many.iter().map(String::as_str).collect::<Vec<_>>();
        assert_eq!(interleave(vec![page(&refs)]).len(), PAGE_CAP);
    }
}
