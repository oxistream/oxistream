use axum::Json;
use axum::extract::{Path, Query, State};
use oxistream_db::VideoUpsert;
use rustypipe::model as rp;
use serde::Deserialize;
use utoipa::IntoParams;
use utoipa_axum::router::OpenApiRouter;
use utoipa_axum::routes;

use super::storyboard;
use crate::AppState;
use crate::api::dto::{ChannelDetails, Resolved, SearchPage, VideoDetails, VideoPage};
use crate::api::error::{ApiError, ErrorBody};
use crate::api::lang::ContentLanguage;

pub fn router() -> OpenApiRouter<AppState> {
    OpenApiRouter::new()
        .routes(routes!(video))
        .routes(routes!(search))
        .routes(routes!(search_suggestions))
        .routes(routes!(channel))
        .routes(routes!(channel_videos))
        .routes(routes!(resolve))
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
    Ok(Json(video))
}

fn video_record(details: &rp::VideoDetails, player: Option<&rp::VideoPlayer>) -> VideoUpsert {
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

#[derive(Debug, Deserialize, IntoParams)]
#[into_params(parameter_in = Query)]
struct SearchParams {
    /// Search query; required unless `continuation` is given.
    q: Option<String>,
    /// Token from a previous page.
    continuation: Option<String>,
}

/// Search videos, channels and playlists.
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
        (status = BAD_REQUEST, description = "Missing query", body = ErrorBody),
        (status = BAD_GATEWAY, description = "YouTube request failed", body = ErrorBody),
        (status = SERVICE_UNAVAILABLE, description = "Rate limited by YouTube", body = ErrorBody),
    )
)]
async fn search(
    State(state): State<AppState>,
    Query(params): Query<SearchParams>,
    ContentLanguage(lang): ContentLanguage,
) -> Result<Json<SearchPage>, ApiError> {
    let img = |url: &str| state.image_url(url);
    if let Some(token) = non_empty(params.continuation.as_deref()) {
        let page = state.youtube.search_continuation(token, lang).await?;
        return Ok(Json(SearchPage::from_rp(page, None, &img)));
    }
    let query = non_empty(params.q.as_deref())
        .ok_or_else(|| ApiError::bad_request("query parameter `q` is required"))?;
    let result = state.youtube.search(query, lang).await?;
    Ok(Json(SearchPage::from_rp(
        result.items,
        result.corrected_query,
        &img,
    )))
}

#[derive(Debug, Deserialize, IntoParams)]
#[into_params(parameter_in = Query)]
struct SuggestionParams {
    /// Partial search query.
    q: Option<String>,
}

/// Search suggestions for a partial query.
#[utoipa::path(
    get,
    path = "/search/suggestions",
    tag = "youtube",
    params(
        SuggestionParams,
        ("Accept-Language" = Option<String>, Header, description = "Content language"),
    ),
    responses(
        (status = OK, body = Vec<String>),
        (status = BAD_GATEWAY, description = "YouTube request failed", body = ErrorBody),
        (status = SERVICE_UNAVAILABLE, description = "Rate limited by YouTube", body = ErrorBody),
    )
)]
async fn search_suggestions(
    State(state): State<AppState>,
    Query(params): Query<SuggestionParams>,
    ContentLanguage(lang): ContentLanguage,
) -> Result<Json<Vec<String>>, ApiError> {
    let Some(query) = non_empty(params.q.as_deref()) else {
        return Ok(Json(Vec::new()));
    };
    Ok(Json(state.youtube.search_suggestions(query, lang).await?))
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
) -> Result<Json<ChannelDetails>, ApiError> {
    let channel = state.youtube.channel_videos(&id, lang).await?;
    Ok(Json(ChannelDetails::from_rp(channel, &|url| {
        state.image_url(url)
    })))
}

#[derive(Debug, Deserialize, IntoParams)]
#[into_params(parameter_in = Query)]
struct ContinuationParams {
    /// Token from a previous page.
    continuation: Option<String>,
}

/// A further page of a channel's videos.
#[utoipa::path(
    get,
    path = "/channels/{id}/videos",
    tag = "youtube",
    params(
        ("id" = String, Path, description = "YouTube channel ID (`UC…`)"),
        ContinuationParams,
        ("Accept-Language" = Option<String>, Header, description = "Content language"),
    ),
    responses(
        (status = OK, body = VideoPage),
        (status = BAD_REQUEST, description = "Invalid channel ID", body = ErrorBody),
        (status = NOT_FOUND, description = "Not found or unavailable", body = ErrorBody),
        (status = BAD_GATEWAY, description = "YouTube request failed", body = ErrorBody),
        (status = SERVICE_UNAVAILABLE, description = "Rate limited by YouTube", body = ErrorBody),
    )
)]
async fn channel_videos(
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
                .channel_videos_continuation(token, lang)
                .await?
        }
        None => state.youtube.channel_videos(&id, lang).await?.content,
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
