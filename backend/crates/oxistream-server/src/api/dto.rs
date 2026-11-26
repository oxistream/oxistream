//! API representations of YouTube data. Every image URL passes through the `img` mapper.

use rustypipe::model::paginator::Paginator;
use rustypipe::model::richtext::ToPlaintext;
use rustypipe::model::{self as rp, UrlTarget};
use serde::Serialize;
use time::OffsetDateTime;
use utoipa::ToSchema;

use crate::proxy::subtitle_id;

#[derive(Debug, Clone, PartialEq, Eq, Serialize, ToSchema)]
pub struct Thumbnail {
    pub url: String,
    pub width: u32,
    pub height: u32,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, ToSchema)]
pub struct ChannelRef {
    pub id: String,
    pub name: String,
    pub avatar: Vec<Thumbnail>,
    pub subscriber_count: Option<u64>,
    pub verified: bool,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, ToSchema)]
pub struct VideoDetails {
    pub id: String,
    pub title: String,
    /// Plain text; links are written out as URLs.
    pub description: String,
    pub channel: ChannelRef,
    pub view_count: u64,
    pub like_count: Option<u32>,
    #[serde(with = "time::serde::rfc3339::option")]
    pub published_at: Option<OffsetDateTime>,
    /// Localized, human-readable publish date as shown by YouTube.
    pub published_text: Option<String>,
    /// Missing when the player data could not be fetched.
    pub duration_seconds: Option<u32>,
    pub thumbnails: Vec<Thumbnail>,
    pub is_live: bool,
    pub keywords: Vec<String>,
    /// Empty when the player data could not be fetched.
    pub subtitles: Vec<Subtitle>,
    pub related: Vec<VideoSummary>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, ToSchema)]
pub struct Subtitle {
    /// Unique per video: the language code, prefixed with `a.` if auto-generated.
    pub id: String,
    /// Language code, e.g. `en` or `pt-BR`.
    pub lang: String,
    /// Human-readable language name.
    pub label: String,
    pub auto_generated: bool,
    /// `WebVTT` file via the proxy.
    pub url: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, ToSchema)]
pub struct VideoSummary {
    pub id: String,
    pub title: String,
    pub duration_seconds: Option<u32>,
    pub thumbnails: Vec<Thumbnail>,
    pub channel: Option<ChannelRef>,
    #[serde(with = "time::serde::rfc3339::option")]
    pub published_at: Option<OffsetDateTime>,
    pub published_text: Option<String>,
    pub view_count: Option<u64>,
    pub is_live: bool,
    pub is_short: bool,
    pub is_upcoming: bool,
    pub description: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, ToSchema)]
pub struct ChannelSummary {
    pub id: String,
    pub name: String,
    pub handle: Option<String>,
    pub avatar: Vec<Thumbnail>,
    pub subscriber_count: Option<u64>,
    pub verified: bool,
    pub description: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, ToSchema)]
pub struct PlaylistSummary {
    pub id: String,
    pub title: String,
    pub thumbnails: Vec<Thumbnail>,
    pub channel: Option<ChannelRef>,
    pub video_count: Option<u64>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, ToSchema)]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum SearchItem {
    Video(VideoSummary),
    Channel(ChannelSummary),
    Playlist(PlaylistSummary),
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, ToSchema)]
pub struct SearchPage {
    pub items: Vec<SearchItem>,
    /// Pass to `continuation` to fetch the next page; absent on the last page.
    pub continuation: Option<String>,
    /// Set when YouTube searched for a corrected query instead.
    pub corrected_query: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, ToSchema)]
pub struct VideoPage {
    pub items: Vec<VideoSummary>,
    pub continuation: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, ToSchema)]
pub struct ChannelDetails {
    pub id: String,
    pub name: String,
    pub handle: Option<String>,
    pub description: String,
    pub subscriber_count: Option<u64>,
    pub video_count: Option<u64>,
    pub avatar: Vec<Thumbnail>,
    pub banner: Vec<Thumbnail>,
    pub verified: bool,
    pub tags: Vec<String>,
    pub videos: VideoPage,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, ToSchema)]
#[serde(rename_all = "snake_case")]
pub enum ResolvedKind {
    Video,
    Channel,
    Playlist,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, ToSchema)]
pub struct Resolved {
    pub kind: ResolvedKind,
    pub id: String,
    /// Start offset in seconds for video links with a timestamp.
    pub start_seconds: Option<u32>,
}

impl Resolved {
    /// `None` for targets Oxistream does not serve, like YouTube Music albums.
    pub fn from_target(target: UrlTarget) -> Option<Self> {
        let (kind, id, start_seconds) = match target {
            UrlTarget::Video { id, start_time } => {
                (ResolvedKind::Video, id, Some(start_time).filter(|t| *t > 0))
            }
            UrlTarget::Channel { id } => (ResolvedKind::Channel, id, None),
            UrlTarget::Playlist { id } => (ResolvedKind::Playlist, id, None),
            UrlTarget::Album { .. } => return None,
        };
        Some(Self {
            kind,
            id,
            start_seconds,
        })
    }
}

pub fn thumbnails(src: &[rp::Thumbnail], img: &impl Fn(&str) -> String) -> Vec<Thumbnail> {
    src.iter()
        .map(|t| Thumbnail {
            url: img(&t.url),
            width: t.width,
            height: t.height,
        })
        .collect()
}

/// Thumbnails derivable from the video ID alone, for when no player data is available.
fn default_thumbnails(video_id: &str, img: &impl Fn(&str) -> String) -> Vec<Thumbnail> {
    [("mqdefault", 320, 180), ("hqdefault", 480, 360)]
        .into_iter()
        .map(|(name, width, height)| Thumbnail {
            url: img(&format!("https://i.ytimg.com/vi/{video_id}/{name}.jpg")),
            width,
            height,
        })
        .collect()
}

impl ChannelRef {
    pub fn from_rp(tag: rp::ChannelTag, img: &impl Fn(&str) -> String) -> Self {
        Self {
            avatar: thumbnails(&tag.avatar, img),
            verified: tag.verification.verified(),
            subscriber_count: tag.subscriber_count,
            id: tag.id,
            name: tag.name,
        }
    }
}

impl VideoDetails {
    /// `subtitle_url` maps a [`Subtitle::id`] to its URL.
    pub fn from_rp(
        details: rp::VideoDetails,
        player: Option<&rp::VideoPlayer>,
        img: &impl Fn(&str) -> String,
        subtitle_url: &impl Fn(&str) -> String,
    ) -> Self {
        let thumbnails = match player {
            Some(player) if !player.details.thumbnail.is_empty() => {
                thumbnails(&player.details.thumbnail, img)
            }
            _ => default_thumbnails(&details.id, img),
        };
        Self {
            description: details.description.to_plaintext(),
            channel: ChannelRef::from_rp(details.channel, img),
            view_count: details.view_count,
            like_count: details.like_count,
            published_at: details.publish_date,
            published_text: details.publish_date_txt,
            duration_seconds: player.map(|p| p.details.duration),
            thumbnails,
            is_live: details.is_live,
            keywords: player
                .map(|p| p.details.keywords.clone())
                .unwrap_or_default(),
            subtitles: player
                .map(|p| {
                    p.subtitles
                        .iter()
                        .map(|s| Subtitle::from_rp(s, subtitle_url))
                        .collect()
                })
                .unwrap_or_default(),
            related: details
                .recommended
                .items
                .into_iter()
                .map(|v| VideoSummary::from_rp(v, img))
                .collect(),
            id: details.id,
            title: details.name,
        }
    }
}

impl Subtitle {
    pub fn from_rp(subtitle: &rp::Subtitle, url: &impl Fn(&str) -> String) -> Self {
        let id = subtitle_id(&subtitle.lang, subtitle.auto_generated);
        Self {
            url: url(&id),
            id,
            lang: subtitle.lang.clone(),
            label: subtitle.lang_name.clone(),
            auto_generated: subtitle.auto_generated,
        }
    }
}

impl VideoSummary {
    pub fn from_rp(video: rp::VideoItem, img: &impl Fn(&str) -> String) -> Self {
        Self {
            thumbnails: thumbnails(&video.thumbnail, img),
            channel: video.channel.map(|c| ChannelRef::from_rp(c, img)),
            duration_seconds: video.duration,
            published_at: video.publish_date,
            published_text: video.publish_date_txt,
            view_count: video.view_count,
            is_live: video.is_live,
            is_short: video.is_short,
            is_upcoming: video.is_upcoming,
            description: video.short_description,
            id: video.id,
            title: video.name,
        }
    }
}

impl SearchItem {
    pub fn from_rp(item: rp::YouTubeItem, img: &impl Fn(&str) -> String) -> Self {
        match item {
            rp::YouTubeItem::Video(video) => Self::Video(VideoSummary::from_rp(video, img)),
            rp::YouTubeItem::Channel(channel) => Self::Channel(ChannelSummary {
                avatar: thumbnails(&channel.avatar, img),
                verified: channel.verification.verified(),
                subscriber_count: channel.subscriber_count,
                description: channel.short_description,
                handle: channel.handle,
                id: channel.id,
                name: channel.name,
            }),
            rp::YouTubeItem::Playlist(playlist) => Self::Playlist(PlaylistSummary {
                thumbnails: thumbnails(&playlist.thumbnail, img),
                channel: playlist.channel.map(|c| ChannelRef::from_rp(c, img)),
                video_count: playlist.video_count,
                id: playlist.id,
                title: playlist.name,
            }),
        }
    }
}

impl SearchPage {
    pub fn from_rp(
        page: Paginator<rp::YouTubeItem>,
        corrected_query: Option<String>,
        img: &impl Fn(&str) -> String,
    ) -> Self {
        Self {
            items: page
                .items
                .into_iter()
                .map(|item| SearchItem::from_rp(item, img))
                .collect(),
            continuation: page.ctoken,
            corrected_query,
        }
    }
}

impl VideoPage {
    pub fn from_rp(page: Paginator<rp::VideoItem>, img: &impl Fn(&str) -> String) -> Self {
        Self {
            items: page
                .items
                .into_iter()
                .map(|v| VideoSummary::from_rp(v, img))
                .collect(),
            continuation: page.ctoken,
        }
    }
}

impl ChannelDetails {
    pub fn from_rp(
        channel: rp::Channel<Paginator<rp::VideoItem>>,
        img: &impl Fn(&str) -> String,
    ) -> Self {
        Self {
            avatar: thumbnails(&channel.avatar, img),
            banner: thumbnails(&channel.banner, img),
            verified: channel.verification.verified(),
            subscriber_count: channel.subscriber_count,
            video_count: channel.video_count,
            videos: VideoPage::from_rp(channel.content, img),
            description: channel.description,
            handle: channel.handle,
            tags: channel.tags,
            id: channel.id,
            name: channel.name,
        }
    }
}

#[cfg(test)]
mod tests {
    use serde::de::DeserializeOwned;
    use serde_json::{Value, json};
    use time::macros::datetime;

    use super::*;

    fn img(url: &str) -> String {
        format!("proxy:{url}")
    }

    fn sub(id: &str) -> String {
        format!("proxy:subtitle/{id}")
    }

    fn from_json<T: DeserializeOwned>(value: Value) -> T {
        serde_json::from_value(value).unwrap()
    }

    fn paginator(items: &Value, ctoken: Option<&str>, endpoint: &str) -> Value {
        json!({ "count": null, "items": items, "ctoken": ctoken, "endpoint": endpoint })
    }

    fn thumb(url: &str) -> Value {
        json!({ "url": url, "width": 120, "height": 90 })
    }

    fn channel_tag() -> Value {
        json!({
            "id": "UCuAXFkgsw1L7xaCfnd5JJOw",
            "name": "Rick Astley",
            "avatar": [thumb("https://yt3.ggpht.com/a")],
            "verification": "verified",
            "subscriber_count": 4_000_000,
        })
    }

    fn video_item(id: &str) -> Value {
        json!({
            "id": id,
            "name": "Related",
            "duration": 212,
            "thumbnail": [thumb(&format!("https://i.ytimg.com/vi/{id}/default.jpg"))],
            "channel": channel_tag(),
            "publish_date": null,
            "publish_date_txt": "1 year ago",
            "view_count": 10,
            "is_live": false,
            "is_short": false,
            "is_upcoming": false,
            "short_description": null,
        })
    }

    fn video_details() -> rp::VideoDetails {
        from_json(json!({
            "id": "dQw4w9WgXcQ",
            "name": "Never Gonna Give You Up",
            "description": [
                { "Text": { "text": "Listen: " } },
                { "Web": { "text": "link", "url": "https://example.com" } },
            ],
            "channel": channel_tag(),
            "view_count": 1_000,
            "like_count": 50,
            "publish_date": "2009-10-25T06:57:33Z",
            "publish_date_txt": "Oct 25, 2009",
            "is_live": false,
            "is_ccommons": false,
            "chapters": [],
            "recommended": paginator(&json!([video_item("aaaaaaaaaaa")]), Some("next"), "next"),
            "top_comments": paginator(&json!([]), None, "next"),
            "latest_comments": paginator(&json!([]), None, "next"),
            "visitor_data": null,
        }))
    }

    fn player() -> rp::VideoPlayer {
        from_json(json!({
            "details": {
                "id": "dQw4w9WgXcQ",
                "name": "Never Gonna Give You Up",
                "description": null,
                "duration": 213,
                "thumbnail": [thumb("https://i.ytimg.com/vi/dQw4w9WgXcQ/hq720.jpg")],
                "channel_id": "UCuAXFkgsw1L7xaCfnd5JJOw",
                "channel_name": "Rick Astley",
                "view_count": 1_000,
                "keywords": ["rick", "astley"],
                "is_live": false,
                "is_live_content": false,
            },
            "video_streams": [],
            "video_only_streams": [],
            "audio_streams": [],
            "subtitles": [
                { "url": "https://www.youtube.com/api/timedtext?lang=de", "lang": "de",
                  "lang_name": "German", "auto_generated": false },
                { "url": "https://www.youtube.com/api/timedtext?lang=en&kind=asr", "lang": "en",
                  "lang_name": "English (auto-generated)", "auto_generated": true },
            ],
            "expires_in_seconds": 21_540,
            "valid_until": "2026-01-01T18:00:00Z",
            "hls_manifest_url": null,
            "dash_manifest_url": null,
            "preview_frames": [],
            "drm": null,
            "client_type": "desktop",
            "visitor_data": null,
        }))
    }

    fn collect_urls(value: &Value, out: &mut Vec<String>) {
        match value {
            Value::Object(map) => {
                if let Some(Value::String(url)) = map.get("url") {
                    out.push(url.clone());
                }
                map.values().for_each(|v| collect_urls(v, out));
            }
            Value::Array(items) => items.iter().for_each(|v| collect_urls(v, out)),
            _ => {}
        }
    }

    fn assert_images_mapped(value: &impl Serialize) {
        let mut urls = Vec::new();
        collect_urls(&serde_json::to_value(value).unwrap(), &mut urls);
        assert_ne!(urls.len(), 0);
        for url in urls {
            assert!(url.starts_with("proxy:"), "unmapped image URL {url}");
        }
    }

    #[test]
    fn maps_video_details_with_player() {
        let player = player();
        let dto = VideoDetails::from_rp(video_details(), Some(&player), &img, &sub);

        assert_eq!(dto.title, "Never Gonna Give You Up");
        assert_eq!(dto.description, "Listen: https://example.com");
        assert_eq!(dto.channel.name, "Rick Astley");
        assert!(dto.channel.verified);
        assert_eq!(dto.channel.subscriber_count, Some(4_000_000));
        assert_eq!(dto.like_count, Some(50));
        assert_eq!(dto.published_at, Some(datetime!(2009-10-25 06:57:33 UTC)));
        assert_eq!(dto.duration_seconds, Some(213));
        assert_eq!(dto.keywords, ["rick", "astley"]);
        assert_eq!(
            dto.thumbnails[0].url,
            "proxy:https://i.ytimg.com/vi/dQw4w9WgXcQ/hq720.jpg"
        );
        assert_eq!(dto.related.len(), 1);
        assert_eq!(dto.related[0].duration_seconds, Some(212));
        assert_eq!(
            dto.subtitles,
            [
                Subtitle {
                    id: "de".into(),
                    lang: "de".into(),
                    label: "German".into(),
                    auto_generated: false,
                    url: "proxy:subtitle/de".into(),
                },
                Subtitle {
                    id: "a.en".into(),
                    lang: "en".into(),
                    label: "English (auto-generated)".into(),
                    auto_generated: true,
                    url: "proxy:subtitle/a.en".into(),
                },
            ]
        );
        assert_images_mapped(&dto);
    }

    #[test]
    fn maps_video_details_without_player() {
        let dto = VideoDetails::from_rp(video_details(), None, &img, &sub);
        assert_eq!(dto.duration_seconds, None);
        assert_eq!(dto.subtitles.len(), 0);
        assert_eq!(dto.keywords.len(), 0);
        assert_eq!(
            dto.thumbnails[1].url,
            "proxy:https://i.ytimg.com/vi/dQw4w9WgXcQ/hqdefault.jpg"
        );
        assert_images_mapped(&dto);
    }

    #[test]
    fn maps_search_items_as_tagged_union() {
        let items = json!([
            { "Video": video_item("aaaaaaaaaaa") },
            { "Channel": {
                "id": "UCuAXFkgsw1L7xaCfnd5JJOw",
                "name": "Rick Astley",
                "handle": "@RickAstleyYT",
                "avatar": [thumb("https://yt3.ggpht.com/a")],
                "verification": "none",
                "subscriber_count": 1,
                "short_description": "",
            } },
            { "Playlist": {
                "id": "PL4lEESSgxM_5O81EvKCmBIm_JT5Q7JeaI",
                "name": "Mix",
                "thumbnail": [thumb("https://i.ytimg.com/vi/x/default.jpg")],
                "channel": null,
                "video_count": 12,
            } },
        ]);
        let page = SearchPage::from_rp(
            from_json(paginator(&items, Some("token"), "search")),
            Some("corrected".into()),
            &img,
        );

        assert_eq!(page.continuation.as_deref(), Some("token"));
        assert_eq!(page.corrected_query.as_deref(), Some("corrected"));
        let json = serde_json::to_value(&page).unwrap();
        let types: Vec<_> = json["items"]
            .as_array()
            .unwrap()
            .iter()
            .map(|item| item["type"].as_str().unwrap())
            .collect();
        assert_eq!(types, ["video", "channel", "playlist"]);
        assert_eq!(json["items"][1]["handle"], "@RickAstleyYT");
        assert_eq!(json["items"][2]["video_count"], 12);
        assert_images_mapped(&page);
    }

    #[test]
    fn maps_channel() {
        let channel: rp::Channel<Paginator<rp::VideoItem>> = from_json(json!({
            "id": "UCuAXFkgsw1L7xaCfnd5JJOw",
            "name": "Rick Astley",
            "handle": "@RickAstleyYT",
            "subscriber_count": 4_000_000,
            "video_count": 300,
            "avatar": [thumb("https://yt3.ggpht.com/a")],
            "verification": "verified",
            "description": "Official channel",
            "tags": ["music"],
            "banner": [thumb("https://yt3.ggpht.com/b")],
            "has_shorts": false,
            "has_live": false,
            "visitor_data": null,
            "content": paginator(&json!([video_item("aaaaaaaaaaa")]), Some("more"), "browse"),
        }));
        let dto = ChannelDetails::from_rp(channel, &img);

        assert_eq!(dto.handle.as_deref(), Some("@RickAstleyYT"));
        assert_eq!(dto.video_count, Some(300));
        assert!(dto.verified);
        assert_eq!(dto.videos.items.len(), 1);
        assert_eq!(dto.videos.continuation.as_deref(), Some("more"));
        assert_images_mapped(&dto);
    }

    #[test]
    fn maps_resolved_targets() {
        let video = |start_time| UrlTarget::Video {
            id: "dQw4w9WgXcQ".into(),
            start_time,
        };
        assert_eq!(
            Resolved::from_target(video(42)),
            Some(Resolved {
                kind: ResolvedKind::Video,
                id: "dQw4w9WgXcQ".into(),
                start_seconds: Some(42),
            })
        );
        assert_eq!(Resolved::from_target(video(0)).unwrap().start_seconds, None);
        assert_eq!(
            Resolved::from_target(UrlTarget::Channel { id: "UC1".into() }).map(|r| r.kind),
            Some(ResolvedKind::Channel)
        );
        assert_eq!(
            Resolved::from_target(UrlTarget::Album {
                id: "MPREb_x".into()
            }),
            None
        );
    }
}
