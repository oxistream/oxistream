use axum::Json;
use axum::extract::rejection::QueryRejection;
use axum::extract::{Path, Query, State};
use oxistream_db::VideoUpsert;
use rustypipe::model as rp;
use rustypipe::param::search_filter::{self as filter, ItemType, SearchFilter};
use rustypipe::param::{ChannelOrder, ChannelVideoTab, Language};
use serde::Deserialize;
use utoipa::{IntoParams, ToSchema};
use utoipa_axum::router::OpenApiRouter;
use utoipa_axum::routes;

use super::blocks::BlockedChannels;
use super::hidden::HiddenVideos;
use super::storyboard;
use crate::AppState;
use crate::api::dto::{
    ChannelDetails, PlaylistPage, Resolved, SearchItem, SearchPage, TrendingCategory,
    TrendingVideos, VideoDetails, VideoPage, YouTubePlaylist,
};
use crate::api::error::{ApiError, ErrorBody};
use crate::api::lang::ContentLanguage;
use crate::youtube::check_channel_id;

pub fn router() -> OpenApiRouter<AppState> {
    OpenApiRouter::new()
        .routes(routes!(video))
        .routes(routes!(search))
        .routes(routes!(search_suggestions))
        .routes(routes!(channel))
        .routes(routes!(channel_videos))
        .routes(routes!(playlist))
        .routes(routes!(playlist_videos))
        .routes(routes!(channel_shorts))
        .routes(routes!(channel_live))
        .routes(routes!(channel_playlists))
        .routes(routes!(channel_search))
        .routes(routes!(resolve))
        .routes(routes!(trending))
}

#[derive(Debug, Deserialize, IntoParams)]
#[into_params(parameter_in = Query)]
struct TrendingParams {
    /// Defaults to `podcasts`.
    #[param(inline)]
    category: Option<TrendingCategory>,
}

/// Trending videos of the instance's region (`OXISTREAM_TRENDING_REGION`).
#[utoipa::path(
    get,
    path = "/trending",
    tag = "youtube",
    params(
        TrendingParams,
        ("Accept-Language" = Option<String>, Header, description = "Content language"),
    ),
    responses(
        (status = OK, body = TrendingVideos),
        (status = BAD_REQUEST, description = "Unknown category", body = ErrorBody),
        (status = BAD_GATEWAY, description = "YouTube request failed", body = ErrorBody),
        (status = SERVICE_UNAVAILABLE, description = "Rate limited by YouTube", body = ErrorBody),
    )
)]
async fn trending(
    State(state): State<AppState>,
    params: Result<Query<TrendingParams>, QueryRejection>,
    ContentLanguage(lang): ContentLanguage,
    blocked: BlockedChannels,
    hidden: HiddenVideos,
) -> Result<Json<TrendingVideos>, ApiError> {
    let Query(params) = params.map_err(|_| ApiError::bad_request("unknown trending `category`"))?;
    let category = params.category.unwrap_or_default();
    let videos = state.youtube.trending(category.into(), lang).await?;
    let mut trending =
        TrendingVideos::from_rp(state.youtube.trending_region(), category, &videos, &|url| {
            state.image_url(url)
        });
    blocked.retain(&mut trending.items);
    hidden.retain(&mut trending.items);
    Ok(Json(trending))
}

/// Video details and related videos.
#[utoipa::path(
    get,
    path = "/videos/{id}",
    tag = "youtube",
    params(
        ("id" = String, Path, description = "YouTube video ID"),
        ("Accept-Language" = Option<String>, Header, description = "Content language"),
    ),
    responses(
        (status = OK, body = VideoDetails),
        (status = BAD_REQUEST, description = "Invalid video ID", body = ErrorBody),
        (status = NOT_FOUND, description = "Not found or unavailable", body = ErrorBody),
        (status = BAD_GATEWAY, description = "YouTube request failed", body = ErrorBody),
        (status = SERVICE_UNAVAILABLE, description = "Rate limited by YouTube", body = ErrorBody),
    )
)]
async fn video(
    State(state): State<AppState>,
    Path(id): Path<String>,
    ContentLanguage(lang): ContentLanguage,
    blocked: BlockedChannels,
) -> Result<Json<VideoDetails>, ApiError> {
    let (details, player) = tokio::join!(
        state.youtube.video_details(&id, lang),
        state.youtube.player(&id)
    );
    let details = details?;
    let player = player
        .inspect_err(|err| tracing::warn!(video_id = %id, error = %err, "player unavailable"))
        .ok();

    let record = video_record(&details, player.as_deref());
    let videos = state.videos.clone();
    tokio::spawn(async move {
        if let Err(err) = videos.upsert(&record).await {
            tracing::warn!(video_id = %record.id, error = ?err, "could not cache video metadata");
        }
    });

    let mut video = VideoDetails::from_rp(
        details,
        player.as_deref(),
        &|url| state.image_url(url),
        &|subtitle| state.subtitle_url(&id, subtitle),
    );
    video.storyboard_url = player.as_deref().and_then(storyboard::track_url);
    blocked.retain(&mut video.related);
    Ok(Json(video))
}

pub(super) fn video_record(
    details: &rp::VideoDetails,
    player: Option<&rp::VideoPlayer>,
) -> VideoUpsert {
    VideoUpsert {
        id: details.id.clone(),
        title: details.name.clone(),
        channel_id: details.channel.id.clone(),
        channel_name: details.channel.name.clone(),
        duration_seconds: player.and_then(|p| i32::try_from(p.details.duration).ok()),
        published_at: details.publish_date,
        thumbnail_url: player.and_then(|p| {
            p.details
                .thumbnail
                .iter()
                .max_by_key(|t| t.width * t.height)
                .map(|t| t.url.clone())
        }),
    }
}

#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Deserialize, ToSchema)]
#[serde(rename_all = "snake_case")]
enum SearchType {
    #[default]
    All,
    Videos,
    Shorts,
    Channels,
    Playlists,
}

impl SearchType {
    fn item_type(self) -> Option<ItemType> {
        match self {
            Self::All => None,
            Self::Videos => Some(ItemType::Video),
            Self::Shorts => Some(ItemType::Short),
            Self::Channels => Some(ItemType::Channel),
            Self::Playlists => Some(ItemType::Playlist),
        }
    }

    /// Typed searches may still return other kinds, which are dropped.
    fn matches(self, item: &SearchItem) -> bool {
        matches!(
            (self, item),
            (Self::All, _)
                | (Self::Videos | Self::Shorts, SearchItem::Video(_))
                | (Self::Channels, SearchItem::Channel(_))
                | (Self::Playlists, SearchItem::Playlist(_))
        )
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Deserialize, ToSchema)]
#[serde(rename_all = "snake_case")]
enum SearchUploadDate {
    Hour,
    Day,
    Week,
    Month,
    Year,
}

impl From<SearchUploadDate> for filter::UploadDate {
    fn from(date: SearchUploadDate) -> Self {
        match date {
            SearchUploadDate::Hour => Self::Hour,
            SearchUploadDate::Day => Self::Day,
            SearchUploadDate::Week => Self::Week,
            SearchUploadDate::Month => Self::Month,
            SearchUploadDate::Year => Self::Year,
        }
    }
}

/// `short` is under 4 minutes, `medium` 4 to 20, `long` over 20.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Deserialize, ToSchema)]
#[serde(rename_all = "snake_case")]
enum SearchDuration {
    Short,
    Medium,
    Long,
}

impl From<SearchDuration> for filter::Length {
    fn from(duration: SearchDuration) -> Self {
        match duration {
            SearchDuration::Short => Self::Short,
            SearchDuration::Medium => Self::Medium,
            SearchDuration::Long => Self::Long,
        }
    }
}

#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Deserialize, ToSchema)]
#[serde(rename_all = "snake_case")]
enum SearchSort {
    #[default]
    Relevance,
    Date,
    Views,
}

impl SearchSort {
    fn order(self) -> Option<filter::Order> {
        match self {
            Self::Relevance => None,
            Self::Date => Some(filter::Order::Date),
            Self::Views => Some(filter::Order::Views),
        }
    }
}

#[derive(Debug, Deserialize, IntoParams)]
#[into_params(parameter_in = Query)]
struct SearchParams {
    /// Search query; required unless `continuation` is given.
    q: Option<String>,
    /// Result type, defaults to `all`. Pass the same type with `continuation`.
    #[param(inline)]
    r#type: Option<SearchType>,
    /// Only results uploaded within this period.
    #[param(inline)]
    upload_date: Option<SearchUploadDate>,
    /// Only videos of this length.
    #[param(inline)]
    duration: Option<SearchDuration>,
    /// Result order, defaults to `relevance`.
    #[param(inline)]
    sort: Option<SearchSort>,
    /// Token from a previous page; it keeps the type and filters of its first page.
    continuation: Option<String>,
}

impl SearchParams {
    /// `None` for an unfiltered search of all types.
    fn filter(&self) -> Option<SearchFilter> {
        let item_type = self.r#type.unwrap_or_default().item_type();
        let order = self.sort.unwrap_or_default().order();
        if item_type.is_none()
            && order.is_none()
            && self.upload_date.is_none()
            && self.duration.is_none()
        {
            return None;
        }
        Some(
            SearchFilter::new()
                .item_type_opt(item_type)
                .sort_opt(order)
                .date_opt(self.upload_date.map(Into::into))
                .length_opt(self.duration.map(Into::into)),
        )
    }
}

#[derive(Debug, Deserialize, IntoParams)]
#[into_params(parameter_in = Query)]
struct ChannelSearchParams {
    /// Search query; required unless `continuation` is given.
    q: Option<String>,
    /// Token from a previous page.
    continuation: Option<String>,
}

/// Search videos, shorts, channels and playlists.
#[utoipa::path(
    get,
    path = "/search",
    tag = "youtube",
    params(
        SearchParams,
        ("Accept-Language" = Option<String>, Header, description = "Content language"),
    ),
    responses(
        (status = OK, body = SearchPage),
        (status = BAD_REQUEST, description = "Missing query, unknown type or filter", body = ErrorBody),
        (status = BAD_GATEWAY, description = "YouTube request failed", body = ErrorBody),
        (status = SERVICE_UNAVAILABLE, description = "Rate limited by YouTube", body = ErrorBody),
    )
)]
async fn search(
    State(state): State<AppState>,
    params: Result<Query<SearchParams>, QueryRejection>,
    ContentLanguage(lang): ContentLanguage,
    blocked: BlockedChannels,
) -> Result<Json<SearchPage>, ApiError> {
    let Query(params) = params.map_err(|err| ApiError::bad_request(err.body_text()))?;
    let kind = params.r#type.unwrap_or_default();
    let img = |url: &str| state.image_url(url);
    let mut page = if let Some(token) = non_empty(params.continuation.as_deref()) {
        let page = state.youtube.search_continuation(token, lang).await?;
        SearchPage::from_rp(page, None, &img)
    } else {
        let query = non_empty(params.q.as_deref())
            .ok_or_else(|| ApiError::bad_request("query parameter `q` is required"))?;
        let filter = params.filter();
        let result = state.youtube.search(query, filter.as_ref(), lang).await?;
        SearchPage::from_rp(result.items, result.corrected_query, &img)
    };
    page.items.retain(|item| kind.matches(item));
    blocked.retain(&mut page.items);
    Ok(Json(page))
}

#[derive(Debug, Deserialize, IntoParams)]
#[into_params(parameter_in = Query)]
struct SuggestionParams {
    /// Partial search query.
    q: Option<String>,
}

/// Search suggestions for a partial query; empty when YouTube fails.
#[utoipa::path(
    get,
    path = "/search/suggestions",
    tag = "youtube",
    params(
        SuggestionParams,
        ("Accept-Language" = Option<String>, Header, description = "Content language"),
    ),
    responses((status = OK, body = Vec<String>))
)]
async fn search_suggestions(
    State(state): State<AppState>,
    Query(params): Query<SuggestionParams>,
    ContentLanguage(lang): ContentLanguage,
) -> Json<Vec<String>> {
    let query = params.q.unwrap_or_default();
    Json(
        state
            .youtube
            .search_suggestions(&query, lang)
            .await
            .to_vec(),
    )
}

/// Channel header and the first page of its videos.
#[utoipa::path(
    get,
    path = "/channels/{id}",
    tag = "youtube",
    params(
        ("id" = String, Path, description = "YouTube channel ID (`UC…`)"),
        ("Accept-Language" = Option<String>, Header, description = "Content language"),
    ),
    responses(
        (status = OK, body = ChannelDetails),
        (status = BAD_REQUEST, description = "Invalid channel ID", body = ErrorBody),
        (status = NOT_FOUND, description = "Not found or unavailable", body = ErrorBody),
        (status = BAD_GATEWAY, description = "YouTube request failed", body = ErrorBody),
        (status = SERVICE_UNAVAILABLE, description = "Rate limited by YouTube", body = ErrorBody),
    )
)]
async fn channel(
    State(state): State<AppState>,
    Path(id): Path<String>,
    ContentLanguage(lang): ContentLanguage,
    blocked: BlockedChannels,
) -> Result<Json<ChannelDetails>, ApiError> {
    let channel = state.youtube.channel_videos(&id, lang).await?;
    let mut details = ChannelDetails::from_rp(channel, &|url| state.image_url(url));
    if blocked.contains(&details.id) {
        details.videos = VideoPage::default();
    }
    Ok(Json(details))
}

#[derive(Debug, Clone, Copy, Default, Deserialize, ToSchema)]
#[serde(rename_all = "snake_case")]
enum ChannelSort {
    #[default]
    Latest,
    Popular,
    Oldest,
}

impl From<ChannelSort> for ChannelOrder {
    fn from(sort: ChannelSort) -> Self {
        match sort {
            ChannelSort::Latest => Self::Latest,
            ChannelSort::Popular => Self::Popular,
            ChannelSort::Oldest => Self::Oldest,
        }
    }
}

#[derive(Debug, Deserialize, IntoParams)]
#[into_params(parameter_in = Query)]
struct ChannelTabParams {
    /// Sort order of the first page; later pages keep it. Defaults to `latest`.
    #[param(inline)]
    sort: Option<ChannelSort>,
    /// Token from a previous page.
    continuation: Option<String>,
}

async fn channel_tab_page(
    state: &AppState,
    id: &str,
    tab: ChannelVideoTab,
    params: Result<Query<ChannelTabParams>, QueryRejection>,
    lang: Language,
    blocked: &BlockedChannels,
) -> Result<Json<VideoPage>, ApiError> {
    let Query(params) = params
        .map_err(|_| ApiError::bad_request("`sort` must be `latest`, `popular` or `oldest`"))?;
    check_channel_id(id)?;
    if blocked.contains(id) {
        return Ok(Json(VideoPage::default()));
    }
    let page = if let Some(token) = non_empty(params.continuation.as_deref()) {
        state
            .youtube
            .channel_videos_continuation(token, lang)
            .await?
    } else {
        let order = params.sort.unwrap_or_default().into();
        state.youtube.channel_tab(id, tab, order, lang).await?
    };
    Ok(Json(VideoPage::from_rp(page, &|url| state.image_url(url))))
}

/// A page of a channel's videos.
#[utoipa::path(
    get,
    path = "/channels/{id}/videos",
    tag = "youtube",
    params(
        ("id" = String, Path, description = "YouTube channel ID (`UC…`)"),
        ChannelTabParams,
        ("Accept-Language" = Option<String>, Header, description = "Content language"),
    ),
    responses(
        (status = OK, body = VideoPage),
        (status = BAD_REQUEST, description = "Invalid channel ID or sort", body = ErrorBody),
        (status = NOT_FOUND, description = "Not found or unavailable", body = ErrorBody),
        (status = BAD_GATEWAY, description = "YouTube request failed", body = ErrorBody),
        (status = SERVICE_UNAVAILABLE, description = "Rate limited by YouTube", body = ErrorBody),
    )
)]
async fn channel_videos(
    State(state): State<AppState>,
    Path(id): Path<String>,
    params: Result<Query<ChannelTabParams>, QueryRejection>,
    ContentLanguage(lang): ContentLanguage,
    blocked: BlockedChannels,
) -> Result<Json<VideoPage>, ApiError> {
    channel_tab_page(&state, &id, ChannelVideoTab::Videos, params, lang, &blocked).await
}

/// A page of a channel's shorts.
#[utoipa::path(
    get,
    path = "/channels/{id}/shorts",
    tag = "youtube",
    params(
        ("id" = String, Path, description = "YouTube channel ID (`UC…`)"),
        ChannelTabParams,
        ("Accept-Language" = Option<String>, Header, description = "Content language"),
    ),
    responses(
        (status = OK, body = VideoPage),
        (status = BAD_REQUEST, description = "Invalid channel ID or sort", body = ErrorBody),
        (status = NOT_FOUND, description = "Not found or unavailable", body = ErrorBody),
        (status = BAD_GATEWAY, description = "YouTube request failed", body = ErrorBody),
        (status = SERVICE_UNAVAILABLE, description = "Rate limited by YouTube", body = ErrorBody),
    )
)]
async fn channel_shorts(
    State(state): State<AppState>,
    Path(id): Path<String>,
    params: Result<Query<ChannelTabParams>, QueryRejection>,
    ContentLanguage(lang): ContentLanguage,
    blocked: BlockedChannels,
) -> Result<Json<VideoPage>, ApiError> {
    channel_tab_page(&state, &id, ChannelVideoTab::Shorts, params, lang, &blocked).await
}

/// A page of a channel's past, current and upcoming livestreams.
#[utoipa::path(
    get,
    path = "/channels/{id}/live",
    tag = "youtube",
    params(
        ("id" = String, Path, description = "YouTube channel ID (`UC…`)"),
        ChannelTabParams,
        ("Accept-Language" = Option<String>, Header, description = "Content language"),
    ),
    responses(
        (status = OK, body = VideoPage),
        (status = BAD_REQUEST, description = "Invalid channel ID or sort", body = ErrorBody),
        (status = NOT_FOUND, description = "Not found or unavailable", body = ErrorBody),
        (status = BAD_GATEWAY, description = "YouTube request failed", body = ErrorBody),
        (status = SERVICE_UNAVAILABLE, description = "Rate limited by YouTube", body = ErrorBody),
    )
)]
async fn channel_live(
    State(state): State<AppState>,
    Path(id): Path<String>,
    params: Result<Query<ChannelTabParams>, QueryRejection>,
    ContentLanguage(lang): ContentLanguage,
    blocked: BlockedChannels,
) -> Result<Json<VideoPage>, ApiError> {
    channel_tab_page(&state, &id, ChannelVideoTab::Live, params, lang, &blocked).await
}

#[derive(Debug, Deserialize, IntoParams)]
#[into_params(parameter_in = Query)]
struct ContinuationParams {
    /// Token from a previous page.
    continuation: Option<String>,
}

/// A page of a channel's playlists.
#[utoipa::path(
    get,
    path = "/channels/{id}/playlists",
    tag = "youtube",
    params(
        ("id" = String, Path, description = "YouTube channel ID (`UC…`)"),
        ContinuationParams,
        ("Accept-Language" = Option<String>, Header, description = "Content language"),
    ),
    responses(
        (status = OK, body = PlaylistPage),
        (status = BAD_REQUEST, description = "Invalid channel ID", body = ErrorBody),
        (status = NOT_FOUND, description = "Not found or unavailable", body = ErrorBody),
        (status = BAD_GATEWAY, description = "YouTube request failed", body = ErrorBody),
        (status = SERVICE_UNAVAILABLE, description = "Rate limited by YouTube", body = ErrorBody),
    )
)]
async fn channel_playlists(
    State(state): State<AppState>,
    Path(id): Path<String>,
    Query(params): Query<ContinuationParams>,
    ContentLanguage(lang): ContentLanguage,
    blocked: BlockedChannels,
) -> Result<Json<PlaylistPage>, ApiError> {
    check_channel_id(&id)?;
    if blocked.contains(&id) {
        return Ok(Json(PlaylistPage::default()));
    }
    let page = match non_empty(params.continuation.as_deref()) {
        Some(token) => {
            state
                .youtube
                .channel_playlists_continuation(token, lang)
                .await?
        }
        None => state.youtube.channel_playlists(&id, lang).await?,
    };
    Ok(Json(PlaylistPage::from_rp(page, &|url| {
        state.image_url(url)
    })))
}

/// Search within a channel's videos.
#[utoipa::path(
    get,
    path = "/channels/{id}/search",
    tag = "youtube",
    params(
        ("id" = String, Path, description = "YouTube channel ID (`UC…`)"),
        ChannelSearchParams,
        ("Accept-Language" = Option<String>, Header, description = "Content language"),
    ),
    responses(
        (status = OK, body = VideoPage),
        (status = BAD_REQUEST, description = "Invalid channel ID or missing query", body = ErrorBody),
        (status = NOT_FOUND, description = "Not found or unavailable", body = ErrorBody),
        (status = BAD_GATEWAY, description = "YouTube request failed", body = ErrorBody),
        (status = SERVICE_UNAVAILABLE, description = "Rate limited by YouTube", body = ErrorBody),
    )
)]
async fn channel_search(
    State(state): State<AppState>,
    Path(id): Path<String>,
    Query(params): Query<ChannelSearchParams>,
    ContentLanguage(lang): ContentLanguage,
    blocked: BlockedChannels,
) -> Result<Json<VideoPage>, ApiError> {
    check_channel_id(&id)?;
    if blocked.contains(&id) {
        return Ok(Json(VideoPage::default()));
    }
    let page = if let Some(token) = non_empty(params.continuation.as_deref()) {
        state
            .youtube
            .channel_videos_continuation(token, lang)
            .await?
    } else {
        let query = non_empty(params.q.as_deref())
            .ok_or_else(|| ApiError::bad_request("query parameter `q` is required"))?;
        state.youtube.channel_search(&id, query, lang).await?
    };
    Ok(Json(VideoPage::from_rp(page, &|url| state.image_url(url))))
}

/// A YouTube playlist with the first page of its videos.
#[utoipa::path(
    get,
    path = "/youtube/playlists/{id}",
    tag = "youtube",
    params(
        ("id" = String, Path, description = "YouTube playlist ID (`PL…`, `UU…`, `OLAK…`, …)"),
        ("Accept-Language" = Option<String>, Header, description = "Content language"),
    ),
    responses(
        (status = OK, body = YouTubePlaylist),
        (status = BAD_REQUEST, description = "Invalid playlist ID", body = ErrorBody),
        (status = NOT_FOUND, description = "Not found, private, or a mix (`reason` is `mix`)", body = ErrorBody),
        (status = BAD_GATEWAY, description = "YouTube request failed", body = ErrorBody),
        (status = SERVICE_UNAVAILABLE, description = "Rate limited by YouTube", body = ErrorBody),
    )
)]
async fn playlist(
    State(state): State<AppState>,
    Path(id): Path<String>,
    ContentLanguage(lang): ContentLanguage,
) -> Result<Json<YouTubePlaylist>, ApiError> {
    let playlist = state.youtube.playlist(&id, lang).await?;
    Ok(Json(YouTubePlaylist::from_rp(playlist, &|url| {
        state.image_url(url)
    })))
}

/// A further page of a YouTube playlist's videos.
#[utoipa::path(
    get,
    path = "/youtube/playlists/{id}/videos",
    tag = "youtube",
    params(
        ("id" = String, Path, description = "YouTube playlist ID"),
        ContinuationParams,
        ("Accept-Language" = Option<String>, Header, description = "Content language"),
    ),
    responses(
        (status = OK, body = VideoPage),
        (status = BAD_REQUEST, description = "Invalid playlist ID", body = ErrorBody),
        (status = NOT_FOUND, description = "Not found, private, or a mix", body = ErrorBody),
        (status = BAD_GATEWAY, description = "YouTube request failed", body = ErrorBody),
        (status = SERVICE_UNAVAILABLE, description = "Rate limited by YouTube", body = ErrorBody),
    )
)]
async fn playlist_videos(
    State(state): State<AppState>,
    Path(id): Path<String>,
    Query(params): Query<ContinuationParams>,
    ContentLanguage(lang): ContentLanguage,
) -> Result<Json<VideoPage>, ApiError> {
    let img = |url: &str| state.image_url(url);
    let page = match non_empty(params.continuation.as_deref()) {
        Some(token) => {
            state
                .youtube
                .playlist_continuation(&id, token, lang)
                .await?
        }
        None => state.youtube.playlist(&id, lang).await?.videos,
    };
    Ok(Json(VideoPage::from_rp(page, &img)))
}

#[derive(Debug, Deserialize, IntoParams)]
#[into_params(parameter_in = Query)]
struct ResolveParams {
    /// YouTube URL, path (`/@handle`, `/watch?v=…`), ID or handle.
    url: Option<String>,
}

/// Resolve a YouTube URL to a video, channel or playlist ID.
#[utoipa::path(
    get,
    path = "/resolve",
    tag = "youtube",
    params(ResolveParams),
    responses(
        (status = OK, body = Resolved),
        (status = BAD_REQUEST, description = "Not a supported YouTube URL", body = ErrorBody),
        (status = NOT_FOUND, description = "Target does not exist", body = ErrorBody),
        (status = BAD_GATEWAY, description = "YouTube request failed", body = ErrorBody),
        (status = SERVICE_UNAVAILABLE, description = "Rate limited by YouTube", body = ErrorBody),
    )
)]
async fn resolve(
    State(state): State<AppState>,
    Query(params): Query<ResolveParams>,
) -> Result<Json<Resolved>, ApiError> {
    let url = non_empty(params.url.as_deref())
        .ok_or_else(|| ApiError::bad_request("query parameter `url` is required"))?;
    let target = state.youtube.resolve(url).await?;
    Resolved::from_target(target)
        .map(Json)
        .ok_or_else(|| ApiError::bad_request("YouTube Music albums are not supported"))
}

fn non_empty(value: Option<&str>) -> Option<&str> {
    value.map(str::trim).filter(|v| !v.is_empty())
}

#[cfg(test)]
mod tests {
    use super::*;

    fn params(query: &str) -> SearchParams {
        let uri = format!("/search?{query}").parse().unwrap();
        Query::<SearchParams>::try_from_uri(&uri).unwrap().0
    }

    #[test]
    fn plain_searches_need_no_filter() {
        assert!(params("q=x").filter().is_none());
        assert!(params("q=x&type=all&sort=relevance").filter().is_none());
    }

    #[test]
    fn filters_map_to_rustypipe() {
        let filter = params("q=x&type=videos&upload_date=week&duration=long&sort=views")
            .filter()
            .unwrap();
        let expected = SearchFilter::new()
            .item_type(ItemType::Video)
            .date(filter::UploadDate::Week)
            .length(filter::Length::Long)
            .sort(filter::Order::Views);
        assert_eq!(format!("{filter:?}"), format!("{expected:?}"));
        assert!(params("q=x&upload_date=hour").filter().is_some());
    }
}
