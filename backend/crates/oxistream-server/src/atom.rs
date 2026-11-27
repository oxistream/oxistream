//! Atom (RFC 4287) rendering of a subscription feed.

use std::borrow::Cow;
use std::io;

use oxistream_db::{FeedVideo, Image};
use quick_xml::Writer;
use quick_xml::escape::escape;
use quick_xml::events::{BytesDecl, BytesText, Event};
use time::OffsetDateTime;
use time::format_description::well_known::Rfc3339;

pub const CONTENT_TYPE: &str = "application/atom+xml; charset=utf-8";
const ATOM_NS: &str = "http://www.w3.org/2005/Atom";
const MEDIA_NS: &str = "http://search.yahoo.com/mrss/";

/// A feed to render. URLs are absolute.
pub struct AtomFeed<'a> {
    /// Permanent ID of the feed.
    pub id: &'a str,
    pub title: &'a str,
    /// The feed's own URL.
    pub self_url: &'a str,
    /// Instance origin without trailing slash, e.g. `https://tube.example.com`.
    pub origin: &'a str,
    pub videos: &'a [FeedVideo],
    /// Maps an upstream image URL to the URL readers load it from.
    pub image_url: &'a dyn Fn(&str) -> String,
}

/// XML 1.0 forbids most control characters even when escaped.
fn xml_safe(text: &str) -> Cow<'_, str> {
    let allowed = |c: char| !c.is_control() || matches!(c, '\t' | '\n' | '\r');
    if text.chars().all(allowed) {
        Cow::Borrowed(text)
    } else {
        Cow::Owned(text.chars().filter(|&c| allowed(c)).collect())
    }
}

fn rfc3339(at: OffsetDateTime) -> String {
    at.format(&Rfc3339).expect("timestamps format as RFC 3339")
}

fn text_element<W: io::Write>(writer: &mut Writer<W>, name: &str, text: &str) -> io::Result<()> {
    writer
        .create_element(name)
        .write_text_content(BytesText::new(&xml_safe(text)))?;
    Ok(())
}

fn largest(images: &[Image]) -> Option<&Image> {
    images.iter().max_by_key(|image| image.width)
}

fn video_path(video: &FeedVideo) -> String {
    if video.is_short {
        format!("/shorts/{}", video.video_id)
    } else {
        format!("/watch?v={}", video.video_id)
    }
}

impl AtomFeed<'_> {
    pub fn render(&self) -> String {
        let mut writer = Writer::new(Vec::new());
        self.write(&mut writer)
            .expect("writing to a Vec cannot fail");
        String::from_utf8(writer.into_inner()).expect("the writer emits UTF-8")
    }

    fn write(&self, writer: &mut Writer<Vec<u8>>) -> io::Result<()> {
        writer.write_event(Event::Decl(BytesDecl::new("1.0", Some("utf-8"), None)))?;
        let updated = self
            .videos
            .iter()
            .map(|video| video.sort_at)
            .max()
            .unwrap_or_else(OffsetDateTime::now_utc);
        writer
            .create_element("feed")
            .with_attribute(("xmlns", ATOM_NS))
            .with_attribute(("xmlns:media", MEDIA_NS))
            .write_inner_content(|w| {
                text_element(w, "id", self.id)?;
                text_element(w, "title", self.title)?;
                text_element(w, "updated", &rfc3339(updated))?;
                w.create_element("link")
                    .with_attribute(("rel", "self"))
                    .with_attribute(("type", "application/atom+xml"))
                    .with_attribute(("href", self.self_url))
                    .write_empty()?;
                w.create_element("link")
                    .with_attribute(("rel", "alternate"))
                    .with_attribute(("type", "text/html"))
                    .with_attribute(("href", &*format!("{}/feed/subscriptions", self.origin)))
                    .write_empty()?;
                text_element(w, "generator", "Oxistream")?;
                for video in self.videos {
                    self.write_entry(w, video)?;
                }
                Ok(())
            })?;
        Ok(())
    }

    fn write_entry(&self, w: &mut Writer<Vec<u8>>, video: &FeedVideo) -> io::Result<()> {
        let link = format!("{}{}", self.origin, video_path(video));
        let title = xml_safe(&video.title);
        let thumbnail =
            largest(&video.thumbnails).map(|image| (image, (self.image_url)(&image.url)));
        let thumbnail = thumbnail.map(|(image, url)| (image, format!("{}{url}", self.origin)));
        w.create_element("entry").write_inner_content(|w| {
            text_element(w, "id", &format!("yt:video:{}", video.video_id))?;
            text_element(w, "title", &title)?;
            w.create_element("link")
                .with_attribute(("rel", "alternate"))
                .with_attribute(("href", link.as_str()))
                .write_empty()?;
            w.create_element("author").write_inner_content(|w| {
                text_element(
                    w,
                    "name",
                    video.channel_name.as_deref().unwrap_or(&video.channel_id),
                )?;
                text_element(
                    w,
                    "uri",
                    &format!("{}/channel/{}", self.origin, video.channel_id),
                )
            })?;
            text_element(
                w,
                "published",
                &rfc3339(video.published_at.unwrap_or(video.sort_at)),
            )?;
            text_element(w, "updated", &rfc3339(video.sort_at))?;
            if let Some((image, url)) = &thumbnail {
                let html = format!(
                    "<a href=\"{}\"><img src=\"{}\" alt=\"{}\" width=\"{}\" height=\"{}\"></a>",
                    escape(link.as_str()),
                    escape(url.as_str()),
                    escape(&*title),
                    image.width,
                    image.height,
                );
                w.create_element("content")
                    .with_attribute(("type", "html"))
                    .write_text_content(BytesText::new(&html))?;
                w.create_element("media:thumbnail")
                    .with_attribute(("url", url.as_str()))
                    .with_attribute(("width", &*image.width.to_string()))
                    .with_attribute(("height", &*image.height.to_string()))
                    .write_empty()?;
            }
            Ok(())
        })?;
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use time::macros::datetime;

    use super::*;

    fn assert_well_formed(xml: &str) {
        let mut reader = quick_xml::Reader::from_str(xml);
        reader.config_mut().check_end_names = true;
        let mut depth = 0i32;
        loop {
            match reader.read_event().expect("well-formed XML") {
                Event::Start(_) => depth += 1,
                Event::End(_) => depth -= 1,
                Event::Eof => break,
                _ => {}
            }
        }
        assert_eq!(depth, 0);
    }

    fn video(id: &str, title: &str, is_short: bool) -> FeedVideo {
        FeedVideo {
            video_id: id.into(),
            channel_id: "UCaaaaaaaaaaaaaaaaaaaaaa".into(),
            channel_name: Some("Tom & Jerry".into()),
            channel_avatar: vec![],
            channel_verified: false,
            title: title.into(),
            duration_seconds: Some(60),
            published_at: None,
            published_text: None,
            thumbnails: vec![
                Image {
                    url: "https://i.ytimg.com/vi/x/default.jpg".into(),
                    width: 120,
                    height: 90,
                },
                Image {
                    url: "https://i.ytimg.com/vi/x/hqdefault.jpg?a=1&b=2".into(),
                    width: 480,
                    height: 360,
                },
            ],
            view_count: None,
            is_live: false,
            is_upcoming: false,
            is_short,
            sort_at: datetime!(2026-11-27 10:00 UTC),
        }
    }

    #[test]
    fn escapes_text_and_strips_control_characters() {
        let videos = [
            video("aaaaaaaaaaa", "<b>\"Fish\" & chips\u{1}</b>", false),
            video("bbbbbbbbbbb", "Short", true),
        ];
        let image_url = |url: &str| format!("/proxy/image/{}?sig=1&exp=2", url.len());
        let xml = AtomFeed {
            id: "urn:uuid:0",
            title: "Subscriptions",
            self_url: "https://tube.example.com/api/v0/feed/atom/t",
            origin: "https://tube.example.com",
            videos: &videos,
            image_url: &image_url,
        }
        .render();

        assert!(!xml.contains('\u{1}'));
        assert!(xml.contains("<title>&lt;b&gt;&quot;Fish&quot; &amp; chips&lt;/b&gt;</title>"));
        assert!(xml.contains("<name>Tom &amp; Jerry</name>"));
        assert!(xml.contains("href=\"https://tube.example.com/watch?v=aaaaaaaaaaa\""));
        assert!(xml.contains("href=\"https://tube.example.com/shorts/bbbbbbbbbbb\""));
        assert!(xml.contains(
            "<media:thumbnail url=\"https://tube.example.com/proxy/image/46?sig=1&amp;exp=2\" \
             width=\"480\" height=\"360\"/>"
        ));
        assert!(xml.contains("<updated>2026-11-27T10:00:00Z</updated>"));

        assert_well_formed(&xml);
        // The HTML content is escaped once more inside the XML text.
        let start = xml.find("<content type=\"html\">").unwrap() + 21;
        let end = start + xml[start..].find("</content>").unwrap();
        let content = quick_xml::escape::unescape(&xml[start..end]).unwrap();
        assert!(content.starts_with("<a href=\"https://tube.example.com/watch?v=aaaaaaaaaaa\">"));
        assert!(content.contains("sig=1&amp;exp=2"));
        assert!(content.contains("alt=\"&lt;b&gt;&quot;Fish&quot; &amp; chips&lt;/b&gt;\""));
    }

    #[test]
    fn empty_feed_is_valid() {
        let xml = AtomFeed {
            id: "urn:uuid:0",
            title: "Subscriptions",
            self_url: "https://tube.example.com/api/v0/feed/atom/t",
            origin: "https://tube.example.com",
            videos: &[],
            image_url: &|url: &str| url.to_owned(),
        }
        .render();
        assert!(xml.starts_with("<?xml version=\"1.0\" encoding=\"utf-8\"?>"));
        assert!(xml.contains("<updated>"));
        assert!(!xml.contains("<entry>"));
        assert_well_formed(&xml);
    }
}
