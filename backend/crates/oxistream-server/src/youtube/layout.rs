//! Reads fields from raw YouTube responses that RustyPipe 0.11.4 loses with the current layout:
//! - channel header parts carrying an `accessibilityLabel` are skipped, so the video count is
//!   reported as the subscriber count (fixed upstream, unreleased);
//! - `lockupViewModel` videos (channel tabs, related videos) moved their duration badge to
//!   `thumbnailBottomOverlayViewModel` and may lack the channel row, so duration, views, dates,
//!   channel and the live flag come back empty.

use std::collections::HashMap;

use rustypipe::model::{ChannelTag, VideoItem};
use rustypipe::param::Language;
use serde_json::{Value, json};

#[derive(Debug, Default, PartialEq, Eq)]
pub(super) struct Fixes {
    /// Present when a channel header was found; overrides RustyPipe's counts.
    pub channel_counts: Option<ChannelCounts>,
    pub videos: HashMap<String, LockupVideo>,
}

#[derive(Debug, Default, Clone, Copy, PartialEq, Eq)]
pub(super) struct ChannelCounts {
    pub subscribers: Option<u64>,
    pub videos: Option<u64>,
}

#[derive(Debug, Default, PartialEq, Eq)]
pub(super) struct LockupVideo {
    pub duration: Option<u32>,
    pub is_live: bool,
    /// Current viewers for livestreams, like RustyPipe's `view_count`.
    pub view_count: Option<u64>,
    pub published_text: Option<String>,
    pub channel: Option<ChannelTag>,
}

impl Fixes {
    pub fn from_bodies(bodies: &[String], lang: Language) -> Self {
        let mut fixes = Self::default();
        for body in bodies {
            let Ok(json) = serde_json::from_str::<Value>(body) else {
                continue;
            };
            if let Some(counts) = channel_counts(&json, lang) {
                fixes.channel_counts = Some(counts);
            }
            collect_lockups(&json, lang, &mut fixes.videos);
        }
        fixes
    }

    pub fn apply(&self, items: &mut [VideoItem]) {
        for item in items {
            let Some(fix) = self.videos.get(&item.id) else {
                continue;
            };
            item.is_live |= fix.is_live;
            if !item.is_live {
                item.duration = item.duration.or(fix.duration);
            }
            item.view_count = item.view_count.or(fix.view_count);
            if item.publish_date_txt.is_none() {
                item.publish_date_txt.clone_from(&fix.published_text);
            }
            if item.channel.is_none() {
                item.channel.clone_from(&fix.channel);
            }
        }
    }
}

fn text(part: &Value) -> Option<&str> {
    part.pointer("/text/content")
        .or_else(|| part.pointer("/avatarStack/avatarStackViewModel/text/content"))
        .and_then(Value::as_str)
}

/// Text parts per metadata row; rows without text (e.g. a "Members only" badge) are dropped.
fn rows(metadata: &Value) -> Vec<Vec<&Value>> {
    metadata
        .pointer("/contentMetadataViewModel/metadataRows")
        .and_then(Value::as_array)
        .into_iter()
        .flatten()
        .map(|row| {
            row["metadataParts"]
                .as_array()
                .into_iter()
                .flatten()
                .filter(|p| text(p).is_some())
                .collect::<Vec<_>>()
        })
        .filter(|parts| !parts.is_empty())
        .collect()
}

fn channel_counts(json: &Value, lang: Language) -> Option<ChannelCounts> {
    let metadata =
        json.pointer("/header/pageHeaderRenderer/content/pageHeaderViewModel/metadata")?;
    let rows = rows(metadata);
    // `@handle` • `N subscribers` • `N videos`, split over one or two rows.
    let parts: Vec<&Value> = rows
        .into_iter()
        .flatten()
        .filter(|p| text(p).is_some_and(|t| !t.starts_with('@')))
        .collect();
    let count = |part: &Value| text(part).and_then(|t| parse_count(t, lang));
    Some(match parts.as_slice() {
        [subs, videos, ..] => ChannelCounts {
            subscribers: count(subs),
            videos: count(videos),
        },
        // Only the subscriber part carries an accessibility label.
        [part] if part.get("accessibilityLabel").is_some() => ChannelCounts {
            subscribers: count(part),
            videos: None,
        },
        [part] => ChannelCounts {
            subscribers: None,
            videos: count(part),
        },
        [] => ChannelCounts::default(),
    })
}

fn collect_lockups(json: &Value, lang: Language, out: &mut HashMap<String, LockupVideo>) {
    match json {
        Value::Object(map) => {
            if let Some(lockup) = map.get("lockupViewModel") {
                if let Some((id, video)) = lockup_video(lockup, lang) {
                    out.insert(id, video);
                }
                return;
            }
            map.values().for_each(|v| collect_lockups(v, lang, out));
        }
        Value::Array(items) => items.iter().for_each(|v| collect_lockups(v, lang, out)),
        _ => {}
    }
}

fn lockup_video(lockup: &Value, lang: Language) -> Option<(String, LockupVideo)> {
    if lockup["contentType"] != "LOCKUP_CONTENT_TYPE_VIDEO" {
        return None;
    }
    let id = lockup["contentId"].as_str()?.to_owned();
    let mut video = LockupVideo::default();

    let badges = lockup
        .pointer("/contentImage/thumbnailViewModel/overlays")
        .and_then(Value::as_array)
        .into_iter()
        .flatten()
        .flat_map(|overlay| {
            [
                overlay.pointer("/thumbnailBottomOverlayViewModel/badges"),
                overlay.pointer("/thumbnailOverlayBadgeViewModel/thumbnailBadges"),
            ]
        })
        .flatten()
        .filter_map(Value::as_array)
        .flatten()
        .filter_map(|badge| badge.get("thumbnailBadgeViewModel"));
    for badge in badges {
        let style = badge["badgeStyle"].as_str().unwrap_or_default();
        if style.ends_with("_LIVE") {
            video.is_live = true;
        } else if video.duration.is_none() {
            video.duration = badge["text"].as_str().and_then(parse_duration);
        }
    }
    if video.is_live {
        video.duration = None;
    }

    let md = &lockup["metadata"]["lockupMetadataViewModel"];
    let avatar = md.pointer("/image/decoratedAvatarViewModel");
    let rows = rows(&md["metadata"]);
    // Channel tabs omit the channel row, related videos start with it.
    let has_channel_row = avatar.is_some() || rows.len() > 1;
    let stats = rows.get(usize::from(has_channel_row));
    match stats.map(Vec::as_slice) {
        Some([viewers, ..]) if video.is_live => {
            video.view_count = text(viewers).and_then(|t| parse_count(t, lang));
        }
        Some([views, published, ..]) => {
            video.view_count = text(views).and_then(|t| parse_count(t, lang));
            video.published_text = text(published).map(str::to_owned);
        }
        _ => {}
    }

    if has_channel_row {
        video.channel = rows
            .first()
            .and_then(|row| row.first())
            .and_then(|part| channel_tag(part, avatar?));
    }
    Some((id, video))
}

fn channel_tag(part: &Value, avatar: &Value) -> Option<ChannelTag> {
    let id = avatar
        .pointer("/rendererContext/commandContext/onTap/innertubeCommand/browseEndpoint/browseId")
        .and_then(Value::as_str)
        .filter(|id| id.starts_with("UC"))?;
    let name = text(part)?.trim();
    let icons: Vec<&str> = part
        .pointer("/text/attachmentRuns")
        .and_then(Value::as_array)
        .into_iter()
        .flatten()
        .filter_map(|run| run.pointer("/element/type/imageType/image/sources"))
        .filter_map(Value::as_array)
        .flatten()
        .filter_map(|src| src.pointer("/clientResource/imageName")?.as_str())
        .collect();
    let verification = if icons.contains(&"AUDIO_BADGE") {
        "artist"
    } else if icons.iter().any(|icon| icon.starts_with("CHECK_CIRCLE")) {
        "verified"
    } else {
        "none"
    };
    let avatar = avatar
        .pointer("/avatar/avatarViewModel/image/sources")
        .cloned()
        .unwrap_or_else(|| json!([]));
    // RustyPipe's models are non-exhaustive and can only be built through serde.
    serde_json::from_value(json!({
        "id": id,
        "name": name,
        "avatar": avatar,
        "verification": verification,
        "subscriber_count": null,
    }))
    .ok()
}

/// Parses `4:04` or `1:02:03`.
pub(super) fn parse_duration(text: &str) -> Option<u32> {
    let parts: Vec<&str> = text.trim().split(':').collect();
    if !(2..=3).contains(&parts.len())
        || parts
            .iter()
            .any(|p| p.is_empty() || !p.bytes().all(|b| b.is_ascii_digit()))
    {
        return None;
    }
    parts.iter().try_fold(0u32, |acc, p| {
        acc.checked_mul(60)?.checked_add(p.parse().ok()?)
    })
}

const THOUSAND: u64 = 1_000;
const MILLION: u64 = 1_000_000;
const BILLION: u64 = 1_000_000_000;

/// Abbreviations YouTube uses for compact numbers, lowercase.
const MULTIPLIERS: &[(&str, u64)] = &[
    ("k", THOUSAND),
    ("tsd", THOUSAND),
    ("mil", THOUSAND),
    ("tys", THOUSAND),
    ("tis", THOUSAND),
    ("tn", THOUSAND),
    ("t", THOUSAND),
    ("e", THOUSAND),
    ("n", THOUSAND),
    ("rb", THOUSAND),
    ("ribu", THOUSAND),
    ("mii", THOUSAND),
    ("тыс", THOUSAND),
    ("тис", THOUSAND),
    ("хил", THOUSAND),
    ("χιλ", THOUSAND),
    ("千", THOUSAND),
    ("천", THOUSAND),
    ("พัน", THOUSAND),
    ("हज़ार", THOUSAND),
    ("ألف", THOUSAND),
    ("万", 10_000),
    ("萬", 10_000),
    ("만", 10_000),
    ("หมื่น", 10_000),
    ("lakh", 100_000),
    ("लाख", 100_000),
    ("แสน", 100_000),
    ("m", MILLION),
    ("mio", MILLION),
    ("mln", MILLION),
    ("mn", MILLION),
    ("mi", MILLION),
    ("mill", MILLION),
    ("milj", MILLION),
    ("jt", MILLION),
    ("tr", MILLION),
    ("млн", MILLION),
    ("εκ", MILLION),
    ("ล้าน", MILLION),
    ("مليون", MILLION),
    ("crore", 10_000_000),
    ("करोड़", 10_000_000),
    ("b", BILLION),
    ("bn", BILLION),
    ("bi", BILLION),
    ("mrd", BILLION),
    ("mld", BILLION),
    ("md", BILLION),
    ("mia", BILLION),
    ("млрд", BILLION),
    ("δισ", BILLION),
    ("مليار", BILLION),
    ("億", 100_000_000),
    ("亿", 100_000_000),
    ("억", 100_000_000),
];

fn multiplier(suffix: &str, lang: Language) -> Option<u64> {
    let suffix = suffix.to_lowercase();
    let (token, mult) = MULTIPLIERS
        .iter()
        .copied()
        .filter(|(token, _)| {
            suffix.strip_prefix(token).is_some_and(|rest| {
                // Latin abbreviations must end at a word boundary ("M views", not "Mal").
                !token.is_ascii() || !rest.starts_with(|c: char| c.is_alphanumeric())
            })
        })
        .max_by_key(|(token, _)| token.len())?;
    Some(match (token, lang) {
        ("b", Language::Tr) => THOUSAND,
        ("mil", Language::Ro) => MILLION,
        _ => mult,
    })
}

/// Parses localized counts like `1,234 views`, `494.763`, `21.5M subscribers`, `3,2 Mio.` or
/// `260万回視聴`. Returns [`None`] rather than a wrongly scaled number when unsure.
pub(super) fn parse_count(text: &str, lang: Language) -> Option<u64> {
    let start = text.find(|c: char| c.is_ascii_digit())?;
    let mut digits = String::new();
    let mut last_sep = None;
    let mut end = text.len();
    let mut chars = text[start..].char_indices().peekable();
    while let Some((i, c)) = chars.next() {
        if c.is_ascii_digit() {
            digits.push(c);
        } else if matches!(c, '.' | ',' | '\'' | '’' | ' ' | '\u{a0}' | '\u{202f}')
            && chars.peek().is_some_and(|(_, n)| n.is_ascii_digit())
        {
            last_sep = Some((c, digits.len()));
        } else {
            end = start + i;
            break;
        }
    }
    let number: u64 = digits.parse().ok()?;
    let fraction_len = last_sep.map_or(0, |(_, pos)| digits.len() - pos);

    match multiplier(text[end..].trim_start(), lang) {
        Some(mult) => {
            let decimals = match last_sep {
                Some(('.' | ',', _)) => u32::try_from(fraction_len).ok()?,
                _ => 0,
            };
            Some(number.checked_mul(mult)? / 10u64.checked_pow(decimals)?)
        }
        // Thousands groups always have three digits; anything else is an unknown abbreviation.
        None if last_sep.is_some() && fraction_len != 3 => None,
        None => Some(number),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parses_durations() {
        assert_eq!(parse_duration("4:04"), Some(244));
        assert_eq!(parse_duration("1:02:03"), Some(3723));
        assert_eq!(parse_duration("10:00:00"), Some(36_000));
        for bad in ["LIVE", "", "4", "1:2:3:4", "a:01", "SHORTS"] {
            assert_eq!(parse_duration(bad), None, "{bad}");
        }
    }

    #[test]
    fn parses_counts() {
        let en = |t| parse_count(t, Language::En);
        assert_eq!(en("21.5M subscribers"), Some(21_500_000));
        assert_eq!(en("1.8K videos"), Some(1_800));
        assert_eq!(en("2.6M views"), Some(2_600_000));
        assert_eq!(en("1,234,567 views"), Some(1_234_567));
        assert_eq!(en("1.2B views"), Some(1_200_000_000));
        assert_eq!(en("2.4K watching"), Some(2_400));
        assert_eq!(en("99 views"), Some(99));
        assert_eq!(en("1 view"), Some(1));
        assert_eq!(en("No views"), None);

        let de = |t| parse_count(t, Language::De);
        assert_eq!(de("3,2 Mio."), Some(3_200_000));
        assert_eq!(de("35 Mio."), Some(35_000_000));
        assert_eq!(de("494.763"), Some(494_763));
        assert_eq!(de("4189 aktive Zuschauer"), Some(4_189));
        assert_eq!(de("21,5 Mio. Abonnenten"), Some(21_500_000));
        assert_eq!(de("1,2 Mrd. Aufrufe"), Some(1_200_000_000));
        assert_eq!(de("8,1\u{a0}Tsd. Videos"), Some(8_100));

        assert_eq!(parse_count("2,6\u{a0}k vues", Language::Fr), Some(2_600));
        assert_eq!(
            parse_count("1\u{202f}234\u{202f}567 vues", Language::Fr),
            Some(1_234_567)
        );
        assert_eq!(
            parse_count("2,6 M de visualizaciones", Language::Es),
            Some(2_600_000)
        );
        assert_eq!(parse_count("26 mil vistas", Language::Es), Some(26_000));
        assert_eq!(
            parse_count("2,6 mil. de vizualizări", Language::Ro),
            Some(2_600_000)
        );
        assert_eq!(parse_count("2,6 B görüntüleme", Language::Tr), Some(2_600));
        assert_eq!(
            parse_count("2,6 млн просмотров", Language::Ru),
            Some(2_600_000)
        );
        assert_eq!(parse_count("260万 回視聴", Language::Ja), Some(2_600_000));
        assert_eq!(parse_count("1.5億回視聴", Language::Ja), Some(150_000_000));
        // Unknown abbreviation with a decimal part: better no number than a wrong one.
        assert_eq!(parse_count("2,6 Xyz views", Language::En), None);
    }

    fn lockup(rows: &Value, badge: (&str, &str), with_avatar: bool) -> Value {
        let mut md = json!({
            "title": { "content": "Title" },
            "metadata": { "contentMetadataViewModel": { "metadataRows": rows } },
        });
        if with_avatar {
            md["image"] = json!({ "decoratedAvatarViewModel": {
                "avatar": { "avatarViewModel": { "image": { "sources": [
                    { "url": "https://yt3.ggpht.com/a", "width": 68, "height": 68 },
                ] } } },
                "rendererContext": { "commandContext": { "onTap": { "innertubeCommand": {
                    "browseEndpoint": { "browseId": "UCeEi1My3KTXHT0SG1JkR9gA" },
                } } } },
            } });
        }
        json!({ "lockupViewModel": {
            "contentId": "djV11Xbc914",
            "contentType": "LOCKUP_CONTENT_TYPE_VIDEO",
            "contentImage": { "thumbnailViewModel": { "overlays": [
                { "thumbnailBottomOverlayViewModel": { "badges": [
                    { "thumbnailBadgeViewModel": { "text": badge.0, "badgeStyle": badge.1 } },
                ] } },
            ] } },
            "metadata": { "lockupMetadataViewModel": md },
        } })
    }

    fn part(text: &str) -> Value {
        json!({ "text": { "content": text }, "accessibilityLabel": text })
    }

    fn fixes(json: &Value, lang: Language) -> Fixes {
        Fixes::from_bodies(&[json.to_string()], lang)
    }

    const DEFAULT: &str = "THUMBNAIL_OVERLAY_BADGE_STYLE_DEFAULT";
    const LIVE: &str = "THUMBNAIL_OVERLAY_BADGE_STYLE_LIVE";

    #[test]
    fn reads_channel_tab_lockups() {
        let body = json!({ "items": [
            lockup(&json!([{ "metadataParts": [part("2.6M views"), part("16 hours ago")] }]),
                ("17:31", DEFAULT), false),
        ] });
        let video = &fixes(&body, Language::En).videos["djV11Xbc914"];
        assert_eq!(
            *video,
            LockupVideo {
                duration: Some(1051),
                is_live: false,
                view_count: Some(2_600_000),
                published_text: Some("16 hours ago".into()),
                channel: None,
            }
        );
    }

    #[test]
    fn ignores_badge_rows() {
        let rows = json!([
            { "metadataParts": [part("2 days ago")] },
            { "badges": [{ "badgeViewModel": { "badgeText": "Members only" } }] },
        ]);
        let video =
            &fixes(&lockup(&rows, ("35:12", DEFAULT), false), Language::En).videos["djV11Xbc914"];
        assert_eq!(video.duration, Some(2112));
        assert_eq!(video.view_count, None);
        assert_eq!(video.channel, None);
    }

    #[test]
    fn reads_related_lockups_with_channel() {
        let mut channel = part("a-ha");
        channel["text"]["attachmentRuns"] = json!([{ "element": { "type": { "imageType": {
            "image": { "sources": [{ "clientResource": { "imageName": "AUDIO_BADGE" } }] },
        } } } }]);
        let rows = json!([
            { "metadataParts": [channel] },
            { "metadataParts": [part("3,2 Mio."), part("vor 10 Monaten")] },
        ]);
        let video = fixes(&lockup(&rows, ("4:04", DEFAULT), true), Language::De)
            .videos
            .remove("djV11Xbc914")
            .unwrap();
        assert_eq!(video.duration, Some(244));
        assert_eq!(video.view_count, Some(3_200_000));
        assert_eq!(video.published_text.as_deref(), Some("vor 10 Monaten"));
        let channel = video.channel.unwrap();
        assert_eq!(channel.id, "UCeEi1My3KTXHT0SG1JkR9gA");
        assert_eq!(channel.name, "a-ha");
        assert_eq!(channel.verification, rustypipe::model::Verification::Artist);
        assert_eq!(channel.avatar[0].url, "https://yt3.ggpht.com/a");
    }

    #[test]
    fn reads_live_lockups() {
        let rows = json!([
            { "metadataParts": [part("Lofi Girl")] },
            { "metadataParts": [part("4189 aktive Zuschauer")] },
        ]);
        let fixes = fixes(&lockup(&rows, ("LIVE", LIVE), true), Language::De);
        let video = &fixes.videos["djV11Xbc914"];
        assert!(video.is_live);
        assert_eq!(video.duration, None);
        assert_eq!(video.view_count, Some(4_189));
        assert_eq!(video.published_text, None);
    }

    #[test]
    fn reads_channel_header_counts() {
        let header = |rows: Value| {
            json!({ "header": { "pageHeaderRenderer": { "content": { "pageHeaderViewModel": {
                "metadata": { "contentMetadataViewModel": { "metadataRows": rows } },
            } } } } })
        };
        let videos = json!({ "text": { "content": "1.8K videos" } });
        let counts = |json: Value| fixes(&json, Language::En).channel_counts.unwrap();

        assert_eq!(
            counts(header(json!([
                { "metadataParts": [{ "text": { "content": "@mkbhd" } }] },
                { "metadataParts": [part("21.5M subscribers"), videos] },
            ]))),
            ChannelCounts {
                subscribers: Some(21_500_000),
                videos: Some(1_800),
            }
        );
        assert_eq!(
            counts(header(json!([
                { "metadataParts": [{ "text": { "content": "@mkbhd" } }, part("21.5M subscribers"), videos] },
            ]))),
            ChannelCounts {
                subscribers: Some(21_500_000),
                videos: Some(1_800),
            }
        );
        assert_eq!(
            counts(header(json!([
                { "metadataParts": [{ "text": { "content": "@hidden" } }] },
                { "metadataParts": [videos] },
            ]))),
            ChannelCounts {
                subscribers: None,
                videos: Some(1_800),
            }
        );
        assert_eq!(fixes(&json!({}), Language::En).channel_counts, None);
    }

    #[test]
    fn fills_only_missing_fields() {
        let body = json!([lockup(
            &json!([{ "metadataParts": [part("2.6M views"), part("16 hours ago")] }]),
            ("17:31", DEFAULT),
            false,
        )]);
        let mut items: Vec<VideoItem> = serde_json::from_value(json!([
            { "id": "djV11Xbc914", "name": "a", "duration": null, "thumbnail": [],
              "channel": null, "publish_date": null, "publish_date_txt": null,
              "view_count": 5, "is_live": false, "is_short": false, "is_upcoming": false,
              "short_description": null },
            { "id": "other", "name": "b", "duration": null, "thumbnail": [],
              "channel": null, "publish_date": null, "publish_date_txt": null,
              "view_count": null, "is_live": false, "is_short": false, "is_upcoming": false,
              "short_description": null },
        ]))
        .unwrap();
        fixes(&body, Language::En).apply(&mut items);
        assert_eq!(items[0].duration, Some(1051));
        assert_eq!(items[0].view_count, Some(5));
        assert_eq!(items[0].publish_date_txt.as_deref(), Some("16 hours ago"));
        assert_eq!(items[1].duration, None);
    }
}
