use std::collections::HashSet;

use axum::Json;
use axum::extract::{FromRequestParts, Path, State};
use axum::http::StatusCode;
use axum::http::request::Parts;
use axum::response::{IntoResponse, Response};
use oxistream_db::{ChannelBlock, Image, NewChannelBlock};
use serde::Serialize;
use time::OffsetDateTime;
use utoipa::ToSchema;
use utoipa_axum::router::OpenApiRouter;
use utoipa_axum::routes;

use crate::AppState;
use crate::api::dto::{PlaylistSummary, SearchItem, Thumbnail, VideoSummary};
use crate::api::error::{ApiError, ErrorBody};
use crate::api::lang::ContentLanguage;
use crate::auth::{CurrentUser, OptionalUser};
use crate::youtube::check_channel_id;

pub fn router() -> OpenApiRouter<AppState> {
    OpenApiRouter::new()
        .routes(routes!(list_blocks))
        .routes(routes!(block, unblock))
}

/// A channel the signed-in user blocked.
#[derive(Debug, Serialize, ToSchema)]
pub struct BlockedChannel {
    pub id: String,
    /// As it was when the channel was blocked.
    pub name: String,
    pub avatar: Vec<Thumbnail>,
    #[serde(with = "time::serde::rfc3339")]
    pub blocked_at: OffsetDateTime,
}

impl BlockedChannel {
    fn from_db(block: ChannelBlock, img: &impl Fn(&str) -> String) -> Self {
        Self {
            avatar: block
                .avatar
                .iter()
                .map(|i| Thumbnail {
                    url: img(&i.url),
                    width: i.width,
                    height: i.height,
                })
                .collect(),
            id: block.channel_id,
            name: block.channel_name,
            blocked_at: block.created_at,
        }
    }
}

/// Channels the requesting user blocked; empty for anonymous requests.
#[derive(Debug, Default)]
pub struct BlockedChannels(HashSet<String>);

/// A listing item that belongs to a channel.
pub trait ChannelItem {
    fn channel_id(&self) -> Option<&str>;
}

impl ChannelItem for VideoSummary {
    fn channel_id(&self) -> Option<&str> {
        self.channel.as_ref().map(|c| c.id.as_str())
    }
}

impl ChannelItem for PlaylistSummary {
    fn channel_id(&self) -> Option<&str> {
        self.channel.as_ref().map(|c| c.id.as_str())
    }
}

impl ChannelItem for SearchItem {
    fn channel_id(&self) -> Option<&str> {
        match self {
            Self::Video(video) => video.channel_id(),
            Self::Channel(channel) => Some(&channel.id),
            Self::Playlist(playlist) => playlist.channel_id(),
        }
    }
}

impl BlockedChannels {
    pub fn contains(&self, channel_id: &str) -> bool {
        self.0.contains(channel_id)
    }

    /// Drops the items of blocked channels. Pages keep their continuation, so a page may come
    /// out short or empty while more follow.
    pub fn retain<T: ChannelItem>(&self, items: &mut Vec<T>) {
        if self.0.is_empty() {
            return;
        }
        items.retain(|item| item.channel_id().is_none_or(|id| !self.contains(id)));
    }
}

impl FromRequestParts<AppState> for BlockedChannels {
    type Rejection = ApiError;

    async fn from_request_parts(parts: &mut Parts, state: &AppState) -> Result<Self, ApiError> {
        let Ok(OptionalUser(Some(user))) = OptionalUser::from_request_parts(parts, state).await
        else {
            return Ok(Self::default());
        };
        Ok(Self(state.blocks.blocked_ids(user.id).await?))
    }
}

/// List blocked channels
///
/// Most recently blocked first.
#[utoipa::path(
    get,
    path = "/blocks",
    tag = "blocks",
    security(("session" = [])),
    responses(
        (status = OK, body = Vec<BlockedChannel>),
        (status = UNAUTHORIZED, description = "Not signed in", body = ErrorBody),
    )
)]
async fn list_blocks(
    State(state): State<AppState>,
    CurrentUser(user): CurrentUser,
) -> Result<Json<Vec<BlockedChannel>>, ApiError> {
    let img = |url: &str| state.image_url(url);
    let blocks = state.blocks.list(user.id).await?;
    Ok(Json(
        blocks
            .into_iter()
            .map(|b| BlockedChannel::from_db(b, &img))
            .collect(),
    ))
}

/// Block channel
///
/// Its videos, shorts and playlists are left out of search, related videos and trending, and a
/// subscription to it ends. Subscribing to the channel again lifts the block. Idempotent.
#[utoipa::path(
    put,
    path = "/blocks/{channel_id}",
    tag = "blocks",
    security(("session" = [])),
    params(
        ("channel_id" = String, Path, description = "YouTube channel ID (`UC…`)"),
        ("Accept-Language" = Option<String>, Header, description = "Content language"),
    ),
    responses(
        (status = CREATED, description = "Blocked", body = BlockedChannel),
        (status = OK, description = "Was already blocked", body = BlockedChannel),
        (status = BAD_REQUEST, description = "Invalid channel ID", body = ErrorBody),
        (status = UNAUTHORIZED, description = "Not signed in", body = ErrorBody),
        (status = FORBIDDEN, description = "Foreign origin", body = ErrorBody),
        (status = NOT_FOUND, description = "No such channel", body = ErrorBody),
        (status = BAD_GATEWAY, description = "YouTube request failed", body = ErrorBody),
        (status = SERVICE_UNAVAILABLE, description = "Rate limited by YouTube", body = ErrorBody),
    )
)]
async fn block(
    State(state): State<AppState>,
    CurrentUser(user): CurrentUser,
    Path(channel_id): Path<String>,
    ContentLanguage(lang): ContentLanguage,
) -> Result<Response, ApiError> {
    check_channel_id(&channel_id)?;
    let img = |url: &str| state.image_url(url);
    if let Some(existing) = state.blocks.get(user.id, &channel_id).await? {
        let body = BlockedChannel::from_db(existing, &img);
        return Ok((StatusCode::OK, Json(body)).into_response());
    }
    let channel = state.youtube.channel_videos(&channel_id, lang).await?;
    let new = NewChannelBlock {
        channel_id,
        channel_name: channel.name,
        avatar: channel
            .avatar
            .into_iter()
            .map(|t| Image {
                url: t.url,
                width: t.width,
                height: t.height,
            })
            .collect(),
    };
    let (block, created) = state.blocks.block(user.id, &new).await?;
    let status = if created {
        StatusCode::CREATED
    } else {
        StatusCode::OK
    };
    Ok((status, Json(BlockedChannel::from_db(block, &img))).into_response())
}

/// Unblock channel
///
/// Succeeds if not blocked, too.
#[utoipa::path(
    delete,
    path = "/blocks/{channel_id}",
    tag = "blocks",
    security(("session" = [])),
    params(("channel_id" = String, Path, description = "YouTube channel ID (`UC…`)")),
    responses(
        (status = NO_CONTENT, description = "Not blocked anymore"),
        (status = BAD_REQUEST, description = "Invalid channel ID", body = ErrorBody),
        (status = UNAUTHORIZED, description = "Not signed in", body = ErrorBody),
        (status = FORBIDDEN, description = "Foreign origin", body = ErrorBody),
    )
)]
async fn unblock(
    State(state): State<AppState>,
    CurrentUser(user): CurrentUser,
    Path(channel_id): Path<String>,
) -> Result<StatusCode, ApiError> {
    check_channel_id(&channel_id)?;
    state.blocks.unblock(user.id, &channel_id).await?;
    Ok(StatusCode::NO_CONTENT)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::api::dto::{ChannelRef, ChannelSummary};

    fn channel(id: &str) -> ChannelRef {
        ChannelRef {
            id: id.into(),
            name: id.into(),
            avatar: Vec::new(),
            subscriber_count: None,
            verified: false,
        }
    }

    fn video(id: &str, channel_id: Option<&str>) -> VideoSummary {
        VideoSummary {
            id: id.into(),
            title: id.into(),
            duration_seconds: None,
            thumbnails: Vec::new(),
            channel: channel_id.map(channel),
            published_at: None,
            published_text: None,
            view_count: None,
            viewers: None,
            is_live: false,
            is_short: false,
            is_upcoming: false,
            description: None,
        }
    }

    #[test]
    fn drops_items_of_blocked_channels() {
        let blocked = BlockedChannels(HashSet::from(["UCblocked".to_owned()]));
        let mut items = vec![
            SearchItem::Video(video("a", Some("UCblocked"))),
            SearchItem::Video(video("b", Some("UCother"))),
            SearchItem::Video(video("c", None)),
            SearchItem::Channel(ChannelSummary {
                id: "UCblocked".into(),
                name: "Blocked".into(),
                handle: None,
                avatar: Vec::new(),
                subscriber_count: None,
                verified: false,
                description: String::new(),
            }),
            SearchItem::Playlist(PlaylistSummary {
                id: "PL1".into(),
                title: "List".into(),
                thumbnails: Vec::new(),
                channel: Some(channel("UCblocked")),
                video_count: None,
                is_course: false,
            }),
        ];
        blocked.retain(&mut items);
        let ids: Vec<_> = items
            .iter()
            .map(|item| match item {
                SearchItem::Video(v) => v.id.as_str(),
                SearchItem::Channel(c) => c.id.as_str(),
                SearchItem::Playlist(p) => p.id.as_str(),
            })
            .collect();
        assert_eq!(ids, ["b", "c"]);
    }

    #[test]
    fn keeps_everything_without_blocks() {
        let mut items = vec![video("a", Some("UCblocked"))];
        BlockedChannels::default().retain(&mut items);
        assert_eq!(items.len(), 1);
    }
}
