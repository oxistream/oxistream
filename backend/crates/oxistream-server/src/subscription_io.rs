//! Subscription list files: YouTube Takeout, NewPipe, Invidious, Piped and OPML.

use std::collections::HashSet;

use oxistream_db::ImportedChannel;
use quick_xml::XmlVersion;
use quick_xml::events::Event;
use reqwest::Url;
use rustypipe::validate;
use serde::{Deserialize, Serialize};
use serde_json::{Value, json};
use utoipa::ToSchema;

const MAX_NAME_CHARS: usize = 200;
const NEWPIPE_YOUTUBE_SERVICE: i64 = 0;
// NewPipe ignores these, but older versions expect them to be present.
const NEWPIPE_APP_VERSION: &str = "0.29.0";
const NEWPIPE_APP_VERSION_INT: u32 = 1014;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, ToSchema)]
#[serde(rename_all = "snake_case")]
pub enum ImportFormat {
    /// `subscriptions.csv` from Google Takeout.
    YoutubeTakeout,
    #[serde(rename = "newpipe")]
    NewPipe,
    /// Invidious JSON export.
    Invidious,
    Piped,
    /// OPML, as exported by Invidious and FreeTube.
    Opml,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ParsedFile {
    pub format: ImportFormat,
    /// Unique channels in file order.
    pub channels: Vec<ImportedChannel>,
    /// Entries that are valid but cannot be imported, such as handles or other services.
    pub skipped: Vec<String>,
    /// Entries that are not channels at all.
    pub invalid: usize,
}

#[derive(Debug, Clone, PartialEq, Eq, thiserror::Error)]
pub enum ParseError {
    #[error("file is not UTF-8 text")]
    NotUtf8,
    #[error("unrecognized file format")]
    Unrecognized,
    #[error("malformed {0} file")]
    Malformed(&'static str),
}

/// Detects the format of a subscription file and extracts its channels.
pub fn parse(body: &[u8]) -> Result<ParsedFile, ParseError> {
    let text = std::str::from_utf8(body).map_err(|_| ParseError::NotUtf8)?;
    let text = text.trim_start_matches('\u{feff}').trim();
    let mut file = Collector::default();
    let format = match text.chars().next() {
        Some('{') => parse_json(text, &mut file)?,
        Some('<') => parse_opml(text, &mut file)?,
        Some(_) => parse_csv(text, &mut file)?,
        None => return Err(ParseError::Unrecognized),
    };
    Ok(ParsedFile {
        format,
        channels: file.channels,
        skipped: file.skipped,
        invalid: file.invalid,
    })
}

enum Entry {
    Channel(String),
    Unsupported,
    Invalid,
}

#[derive(Default)]
struct Collector {
    seen: HashSet<String>,
    channels: Vec<ImportedChannel>,
    skipped: Vec<String>,
    invalid: usize,
}

impl Collector {
    /// Adds an entry given as a channel ID or URL.
    fn add(&mut self, reference: &str, name: Option<&str>) {
        let reference = reference.trim();
        match classify(reference) {
            Entry::Channel(id) => self.add_channel(id, name),
            Entry::Unsupported => self.skip(reference),
            Entry::Invalid => self.invalid += 1,
        }
    }

    fn add_channel(&mut self, id: String, name: Option<&str>) {
        if self.seen.insert(id.clone()) {
            self.channels.push(ImportedChannel {
                id,
                name: name.and_then(clean_name),
            });
        }
    }

    fn skip(&mut self, reference: &str) {
        self.skipped.push(reference.to_owned());
    }
}

fn clean_name(name: &str) -> Option<String> {
    let name = name.trim();
    (!name.is_empty()).then(|| name.chars().take(MAX_NAME_CHARS).collect())
}

fn is_channel_id(id: &str) -> bool {
    validate::channel_id(id).is_ok()
}

/// Only `/channel/UC…` URLs and bare IDs name a channel without a YouTube lookup.
fn classify(reference: &str) -> Entry {
    if is_channel_id(reference) {
        return Entry::Channel(reference.to_owned());
    }
    if reference.starts_with('@') && reference.len() > 1 {
        return Entry::Unsupported;
    }
    let Ok(url) = Url::parse(reference) else {
        return Entry::Invalid;
    };
    if !matches!(url.scheme(), "http" | "https") || url.host_str().is_none() {
        return Entry::Invalid;
    }
    // YouTube RSS feeds, e.g. in OPML.
    if let Some((_, id)) = url.query_pairs().find(|(k, _)| k == "channel_id") {
        return if is_channel_id(&id) {
            Entry::Channel(id.into_owned())
        } else {
            Entry::Invalid
        };
    }
    let segments: Vec<&str> = url
        .path_segments()
        .map(Iterator::collect)
        .unwrap_or_default();
    // `/channel/UC…`, also below a prefix such as Invidious' `/feed/channel/UC…`.
    if let Some(pos) = segments.iter().position(|s| *s == "channel") {
        return match segments.get(pos + 1) {
            Some(id) if is_channel_id(id) => Entry::Channel((*id).to_owned()),
            _ => Entry::Invalid,
        };
    }
    Entry::Unsupported
}

fn parse_json(text: &str, file: &mut Collector) -> Result<ImportFormat, ParseError> {
    let root: Value = serde_json::from_str(text).map_err(|_| ParseError::Malformed("JSON"))?;
    let subscriptions = root
        .get("subscriptions")
        .and_then(Value::as_array)
        .ok_or(ParseError::Unrecognized)?;
    let format = if root.get("format").and_then(Value::as_str) == Some("Piped") {
        ImportFormat::Piped
    } else if !subscriptions.is_empty() && subscriptions.iter().all(Value::is_string) {
        ImportFormat::Invidious
    } else if subscriptions.is_empty() && root.get("app_version").is_none() {
        // An empty Invidious export has no other marker.
        ImportFormat::Invidious
    } else {
        ImportFormat::NewPipe
    };
    for item in subscriptions {
        match item {
            Value::String(id) => file.add(id, None),
            Value::Object(item) => {
                let url = item.get("url").and_then(Value::as_str);
                let name = item.get("name").and_then(Value::as_str);
                let service = item.get("service_id").and_then(Value::as_i64);
                match (url, service) {
                    (Some(url), Some(id)) if id != NEWPIPE_YOUTUBE_SERVICE => file.skip(url),
                    (Some(url), _) => file.add(url, name),
                    (None, _) => file.invalid += 1,
                }
            }
            _ => file.invalid += 1,
        }
    }
    Ok(format)
}

fn parse_opml(text: &str, file: &mut Collector) -> Result<ImportFormat, ParseError> {
    let malformed = |_| ParseError::Malformed("OPML");
    let mut reader = quick_xml::Reader::from_str(text);
    let mut root_seen = false;
    loop {
        let element = match reader.read_event().map_err(malformed)? {
            Event::Start(e) | Event::Empty(e) => e,
            Event::Eof => break,
            _ => continue,
        };
        if !root_seen {
            if element.local_name().as_ref() != "opml" {
                return Err(ParseError::Unrecognized);
            }
            root_seen = true;
            continue;
        }
        if element.local_name().as_ref() != "outline" {
            continue;
        }
        let (mut xml_url, mut title, mut text) = (None, None, None);
        for attr in element.attributes() {
            let attr = attr.map_err(|_| ParseError::Malformed("OPML"))?;
            let value = attr
                .normalized_value(XmlVersion::Implicit1_0)
                .map_err(malformed)?
                .into_owned();
            match attr.key.local_name().as_ref() {
                "xmlUrl" => xml_url = Some(value),
                "title" => title = Some(value),
                "text" => text = Some(value),
                _ => {}
            }
        }
        // Outlines without a feed are folders.
        if let Some(url) = xml_url {
            file.add(&url, title.or(text).as_deref());
        }
    }
    if root_seen {
        Ok(ImportFormat::Opml)
    } else {
        Err(ParseError::Unrecognized)
    }
}

/// Takeout columns are channel ID, URL and title, with localized headers.
fn parse_csv(text: &str, file: &mut Collector) -> Result<ImportFormat, ParseError> {
    let mut records = csv_records(text)
        .into_iter()
        .filter(|r| r.iter().any(|f| !f.trim().is_empty()));
    let Some(first) = records.next() else {
        return Err(ParseError::Unrecognized);
    };
    if first.len() != 3 {
        return Err(ParseError::Unrecognized);
    }
    let first_is_data = is_channel_id(first[0].trim());
    for record in first_is_data.then_some(first).into_iter().chain(records) {
        let name = record.get(2).map(String::as_str);
        let id = record[0].trim();
        if is_channel_id(id) {
            file.add_channel(id.to_owned(), name);
        } else if let Some(url) = record.get(1).filter(|u| !u.trim().is_empty()) {
            file.add(url, name);
        } else {
            file.invalid += 1;
        }
    }
    Ok(ImportFormat::YoutubeTakeout)
}

/// RFC 4180 records; lenient about stray quotes.
fn csv_records(text: &str) -> Vec<Vec<String>> {
    let mut records = Vec::new();
    let mut record = Vec::new();
    let mut field = String::new();
    let mut quoted = false;
    let mut chars = text.chars().peekable();
    while let Some(c) = chars.next() {
        match c {
            '"' if quoted => {
                if chars.peek() == Some(&'"') {
                    chars.next();
                    field.push('"');
                } else {
                    quoted = false;
                }
            }
            '"' if field.is_empty() => quoted = true,
            ',' if !quoted => record.push(std::mem::take(&mut field)),
            '\r' if !quoted => {}
            '\n' if !quoted => {
                record.push(std::mem::take(&mut field));
                records.push(std::mem::take(&mut record));
            }
            c => field.push(c),
        }
    }
    if !field.is_empty() || !record.is_empty() {
        record.push(field);
        records.push(record);
    }
    records
}

/// A channel in an export; `name` falls back to the ID while unknown.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ExportedChannel {
    pub id: String,
    pub name: Option<String>,
}

fn channel_url(id: &str) -> String {
    format!("https://www.youtube.com/channel/{id}")
}

/// NewPipe subscription export, also importable by Invidious, Piped and LibreTube.
pub fn to_newpipe(channels: &[ExportedChannel]) -> String {
    let subscriptions: Vec<Value> = channels
        .iter()
        .map(|c| {
            json!({
                "service_id": NEWPIPE_YOUTUBE_SERVICE,
                "url": channel_url(&c.id),
                "name": c.name.as_deref().unwrap_or(&c.id),
            })
        })
        .collect();
    json!({
        "app_version": NEWPIPE_APP_VERSION,
        "app_version_int": NEWPIPE_APP_VERSION_INT,
        "subscriptions": subscriptions,
    })
    .to_string()
}

/// Invidious data export with subscriptions only.
pub fn to_invidious(channels: &[ExportedChannel]) -> String {
    let ids: Vec<&str> = channels.iter().map(|c| c.id.as_str()).collect();
    json!({
        "subscriptions": ids,
        "watch_history": [],
        "playlists": [],
    })
    .to_string()
}

#[cfg(test)]
mod tests {
    use super::*;

    fn fixture(name: &str) -> Vec<u8> {
        let path = format!(
            "{}/tests/fixtures/subscriptions/{name}",
            env!("CARGO_MANIFEST_DIR")
        );
        std::fs::read(&path).unwrap_or_else(|err| panic!("{path}: {err}"))
    }

    fn ids(file: &ParsedFile) -> Vec<&str> {
        file.channels.iter().map(|c| c.id.as_str()).collect()
    }

    fn name_of<'a>(file: &'a ParsedFile, id: &str) -> Option<&'a str> {
        file.channels
            .iter()
            .find(|c| c.id == id)
            .and_then(|c| c.name.as_deref())
    }

    #[test]
    fn youtube_takeout_csv() {
        let file = parse(&fixture("takeout_en.csv")).unwrap();
        assert_eq!(file.format, ImportFormat::YoutubeTakeout);
        assert_eq!(
            ids(&file),
            [
                "UCsXVk37bltHxD1rDPwtNM8Q",
                "UCHnyfMqiRRG1u-2MsSQLbXA",
                "UC6nSFpj9HTCZ5t-N3Rm3-HA",
                "UCBa659QWEk1AI4Tg--mrJ2A",
            ]
        );
        assert_eq!(
            name_of(&file, "UCsXVk37bltHxD1rDPwtNM8Q"),
            Some("Kurzgesagt – In a Nutshell")
        );
        assert_eq!(
            name_of(&file, "UCBa659QWEk1AI4Tg--mrJ2A"),
            Some("Tom Scott, \"quoted\"")
        );
        assert_eq!((file.skipped.len(), file.invalid), (0, 0));
    }

    #[test]
    fn localized_takeout_csv_with_bom_and_crlf() {
        let file = parse(&fixture("takeout_de.csv")).unwrap();
        assert_eq!(file.format, ImportFormat::YoutubeTakeout);
        assert_eq!(
            ids(&file),
            ["UCsXVk37bltHxD1rDPwtNM8Q", "UC6nSFpj9HTCZ5t-N3Rm3-HA"]
        );
        assert_eq!(file.invalid, 1);
    }

    #[test]
    fn newpipe_json() {
        let file = parse(&fixture("newpipe.json")).unwrap();
        assert_eq!(file.format, ImportFormat::NewPipe);
        assert_eq!(
            ids(&file),
            [
                "UCsXVk37bltHxD1rDPwtNM8Q",
                "UCHnyfMqiRRG1u-2MsSQLbXA",
                "UC6nSFpj9HTCZ5t-N3Rm3-HA",
            ]
        );
        assert_eq!(name_of(&file, "UC6nSFpj9HTCZ5t-N3Rm3-HA"), Some("Vsauce"));
        assert_eq!(
            file.skipped,
            [
                "https://www.youtube.com/@veritasium",
                "https://www.youtube.com/user/vsauce",
                "https://soundcloud.com/lofi-girl",
            ]
        );
        assert_eq!(file.invalid, 1);
    }

    #[test]
    fn invidious_json() {
        let file = parse(&fixture("invidious.json")).unwrap();
        assert_eq!(file.format, ImportFormat::Invidious);
        assert_eq!(
            ids(&file),
            ["UCsXVk37bltHxD1rDPwtNM8Q", "UCHnyfMqiRRG1u-2MsSQLbXA"]
        );
        assert!(file.channels.iter().all(|c| c.name.is_none()));
        assert_eq!(file.invalid, 1, "malformed ID");
    }

    #[test]
    fn invidious_opml() {
        let file = parse(&fixture("invidious.opml")).unwrap();
        assert_eq!(file.format, ImportFormat::Opml);
        assert_eq!(
            ids(&file),
            [
                "UCsXVk37bltHxD1rDPwtNM8Q",
                "UCHnyfMqiRRG1u-2MsSQLbXA",
                "UC6nSFpj9HTCZ5t-N3Rm3-HA",
            ]
        );
        assert_eq!(
            name_of(&file, "UCHnyfMqiRRG1u-2MsSQLbXA"),
            Some("Veritasium & Friends")
        );
    }

    #[test]
    fn piped_json() {
        let file = parse(&fixture("piped.json")).unwrap();
        assert_eq!(file.format, ImportFormat::Piped);
        assert_eq!(
            ids(&file),
            ["UCsXVk37bltHxD1rDPwtNM8Q", "UCBa659QWEk1AI4Tg--mrJ2A"]
        );
        assert_eq!(
            name_of(&file, "UCBa659QWEk1AI4Tg--mrJ2A"),
            Some("Tom Scott")
        );
        assert_eq!(file.skipped, ["https://www.youtube.com/c/LinusTechTips"]);
    }

    #[test]
    fn deduplicates_and_keeps_first_name() {
        let file = parse(
            br#"{"subscriptions":[
                {"service_id":0,"url":"https://www.youtube.com/channel/UCsXVk37bltHxD1rDPwtNM8Q","name":"First"},
                {"service_id":0,"url":"https://youtube.com/channel/UCsXVk37bltHxD1rDPwtNM8Q","name":"Second"}
            ]}"#,
        )
        .unwrap();
        assert_eq!(file.channels.len(), 1);
        assert_eq!(file.channels[0].name.as_deref(), Some("First"));
    }

    #[test]
    fn rejects_unknown_files() {
        for body in [
            &b""[..],
            b"   ",
            b"hello world",
            b"{\"foo\": []}",
            b"{not json",
            b"<html><body></body></html>",
            b"<opml><body><outline",
            b"\xff\xfe\x00",
        ] {
            assert!(parse(body).is_err(), "{}", String::from_utf8_lossy(body));
        }
    }

    #[test]
    fn classifies_references() {
        let id = "UCsXVk37bltHxD1rDPwtNM8Q";
        for channel in [
            id.to_owned(),
            format!("https://www.youtube.com/channel/{id}"),
            format!("http://m.youtube.com/channel/{id}/videos"),
            format!("https://invidious.example/feed/channel/{id}"),
            format!("https://www.youtube.com/feeds/videos.xml?channel_id={id}"),
        ] {
            assert!(
                matches!(classify(&channel), Entry::Channel(c) if c == id),
                "{channel}"
            );
        }
        for unsupported in [
            "@handle",
            "https://www.youtube.com/@handle",
            "https://youtube.com/c/name",
        ] {
            assert!(
                matches!(classify(unsupported), Entry::Unsupported),
                "{unsupported}"
            );
        }
        for invalid in [
            "",
            "UCshort",
            "not a url",
            "ftp://youtube.com/channel/UCsXVk37bltHxD1rDPwtNM8Q",
            "https://www.youtube.com/channel/nope",
        ] {
            assert!(matches!(classify(invalid), Entry::Invalid), "{invalid}");
        }
    }

    #[test]
    fn exports_round_trip() {
        let channels = [
            ExportedChannel {
                id: "UCsXVk37bltHxD1rDPwtNM8Q".into(),
                name: Some("Kurzgesagt".into()),
            },
            ExportedChannel {
                id: "UCHnyfMqiRRG1u-2MsSQLbXA".into(),
                name: None,
            },
        ];
        let newpipe = to_newpipe(&channels);
        let value: Value = serde_json::from_str(&newpipe).unwrap();
        assert_eq!(
            value["subscriptions"][1]["name"],
            "UCHnyfMqiRRG1u-2MsSQLbXA"
        );
        assert_eq!(value["subscriptions"][0]["service_id"], 0);
        let file = parse(newpipe.as_bytes()).unwrap();
        assert_eq!(file.format, ImportFormat::NewPipe);
        assert_eq!(
            name_of(&file, "UCsXVk37bltHxD1rDPwtNM8Q"),
            Some("Kurzgesagt")
        );

        let file = parse(to_invidious(&channels).as_bytes()).unwrap();
        assert_eq!(file.format, ImportFormat::Invidious);
        assert_eq!(
            ids(&file),
            ["UCsXVk37bltHxD1rDPwtNM8Q", "UCHnyfMqiRRG1u-2MsSQLbXA"]
        );
        assert_eq!(
            parse(to_invidious(&[]).as_bytes()).unwrap().format,
            ImportFormat::Invidious
        );
    }
}
