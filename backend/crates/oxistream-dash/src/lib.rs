//! Pure DASH MPD manifest generation. No web or IO dependencies.
//!
//! Builds a static, on-demand MPD from YouTube adaptive streams whose URLs
//! already point to the proxy.

mod model;

use std::collections::{BTreeMap, HashSet};
use std::io;
use std::ops::Range;
use std::time::Duration;

use quick_xml::Writer;
use quick_xml::events::{BytesDecl, BytesText, Event};

pub use model::{AudioStream, AudioTrack, AudioTrackType, VideoStream};

const MPD_NS: &str = "urn:mpeg:dash:schema:mpd:2011";
const PROFILE: &str = "urn:mpeg:dash:profile:isoff-on-demand:2011";
const ROLE_SCHEME: &str = "urn:mpeg:dash:role:2011";
const OXISTREAM_ROLE_SCHEME: &str = "urn:oxistream:dash:role:2026";
const CHANNEL_SCHEME: &str = "urn:mpeg:dash:23003:3:audio_channel_configuration:2011";
const CICP_PRIMARIES: &str = "urn:mpeg:mpegB:cicp:ColourPrimaries";
const CICP_TRANSFER: &str = "urn:mpeg:mpegB:cicp:TransferCharacteristics";
const CICP_MATRIX: &str = "urn:mpeg:mpegB:cicp:MatrixCoefficients";

/// Errors returned by MPD generation.
#[derive(Debug, thiserror::Error)]
pub enum Error {
    /// No stream had the byte ranges required for `SegmentBase`, or all were DRM-protected.
    #[error("no playable streams")]
    NoPlayableStreams,
    #[error(transparent)]
    Io(#[from] io::Error),
}

/// Generates an MPD manifest as a string.
pub fn generate(
    duration: Duration,
    video: &[VideoStream],
    audio: &[AudioStream],
) -> Result<String, Error> {
    let mut buf = Vec::new();
    write(&mut buf, duration, video, audio)?;
    Ok(String::from_utf8(buf).expect("quick-xml writes valid UTF-8"))
}

/// Writes an MPD manifest to `out`.
///
/// Streams without init/index ranges (OTF, live) and DRM-protected streams are skipped.
pub fn write<W: io::Write>(
    out: W,
    duration: Duration,
    video: &[VideoStream],
    audio: &[AudioStream],
) -> Result<(), Error> {
    let sets = build_sets(video, audio);
    if sets.is_empty() {
        return Err(Error::NoPlayableStreams);
    }

    let mut w = Writer::new_with_indent(out, b' ', 2);
    w.write_event(Event::Decl(BytesDecl::new("1.0", Some("UTF-8"), None)))?;
    w.create_element("MPD")
        .with_attributes([
            ("xmlns", MPD_NS),
            ("profiles", PROFILE),
            ("type", "static"),
            ("mediaPresentationDuration", &iso_duration(duration)),
            ("minBufferTime", "PT1.5S"),
        ])
        .write_inner_content(|w| {
            w.create_element("Period")
                .write_inner_content(|w| {
                    for (id, set) in sets.iter().enumerate() {
                        write_set(w, id, set)?;
                    }
                    Ok(())
                })
                .map(|_| ())
        })?;
    Ok(())
}

/// Converts a YouTube byte range to the inclusive `first-last` form DASH expects.
///
/// YouTube's `initRange`/`indexRange` already carry an inclusive `end` (e.g. init
/// `0-740` is followed by index `741-1204`) and RustyPipe copies them verbatim into
/// `Range<u32>`, so `end` is used as-is despite `Range` being nominally exclusive.
fn dash_byte_range(range: &Range<u32>) -> Option<String> {
    (range.start <= range.end).then(|| format!("{}-{}", range.start, range.end))
}

fn iso_duration(duration: Duration) -> String {
    let ms = duration.as_millis();
    let (secs, frac) = (ms / 1000, ms % 1000);
    if frac == 0 {
        format!("PT{secs}S")
    } else {
        format!("PT{secs}.{frac:03}S")
    }
}

/// Splits `video/mp4; codecs="avc1.640028"` into mime type and codecs.
fn parse_mime(mime: &str) -> Option<(&str, &str)> {
    let (mime_type, params) = mime.split_once(';')?;
    let codecs = params.trim().strip_prefix("codecs=")?.trim_matches('"');
    (!codecs.is_empty()).then_some((mime_type.trim(), codecs))
}

/// Normalized codec family and its sort rank (more compatible first).
fn codec_family(codecs: &str) -> (u8, String) {
    let prefix = codecs.split([',', '.']).next().unwrap_or_default().trim();
    let family = match prefix {
        "vp09" => "vp9",
        other => other,
    };
    let rank = match family {
        "avc1" | "mp4a" => 0,
        "vp9" | "opus" => 1,
        "av01" | "ac-3" => 2,
        "ec-3" => 3,
        _ => 9,
    };
    (rank, family.to_owned())
}

/// CICP colour description (ISO/IEC 23091-2) of an HDR stream.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
struct Cicp {
    primaries: u8,
    transfer: u8,
    matrix: u8,
}

impl Cicp {
    /// BT.2020 primaries, PQ transfer, BT.2020 non-constant luminance matrix.
    const HDR10: Self = Self {
        primaries: 9,
        transfer: 16,
        matrix: 9,
    };

    /// Reads the colour fields from full-form `vp09`/`av01` codec strings
    /// (e.g. `vp09.02.51.10.01.09.16.09.00`), falling back to HDR10 (PQ), which
    /// is what YouTube serves for most HDR uploads.
    fn from_codecs(codecs: &str) -> Self {
        let parts: Vec<&str> = codecs.split('.').collect();
        let offset = match parts.first() {
            Some(&"vp09") => 5,
            Some(&"av01") => 6,
            _ => return Self::HDR10,
        };
        let field = |i: usize| parts.get(offset + i).and_then(|p| p.parse().ok());
        match (field(0), field(1), field(2)) {
            (Some(primaries), Some(transfer), Some(matrix)) => Self {
                primaries,
                transfer,
                matrix,
            },
            _ => Self::HDR10,
        }
    }
}

#[derive(Debug, PartialEq, Eq, PartialOrd, Ord)]
struct SetKey {
    is_audio: bool,
    not_default: bool,
    track_id: String,
    codec_rank: u8,
    family: String,
    mime_type: String,
    cicp: Option<Cicp>,
}

struct Set<'a> {
    mime_type: String,
    cicp: Option<Cicp>,
    track: Option<&'a AudioTrack>,
    reps: Vec<Rep<'a>>,
}

struct Rep<'a> {
    id: String,
    bandwidth: u32,
    codecs: &'a str,
    url: &'a str,
    init: String,
    index: String,
    kind: RepKind,
}

enum RepKind {
    Video {
        width: u32,
        height: u32,
        fps: u8,
    },
    Audio {
        channels: Option<u8>,
        sample_rate: Option<u32>,
    },
}

struct Common<'a> {
    itag: u32,
    bitrate: u32,
    mime: &'a str,
    url: &'a str,
    init: Option<&'a Range<u32>>,
    index: Option<&'a Range<u32>>,
    drm: bool,
}

impl<'a> Common<'a> {
    fn rep(&self, id: String, kind: RepKind) -> Option<(&'a str, Rep<'a>)> {
        if self.drm {
            return None;
        }
        let (mime_type, codecs) = parse_mime(self.mime)?;
        let rep = Rep {
            id,
            bandwidth: self.bitrate,
            codecs,
            url: self.url,
            init: dash_byte_range(self.init?)?,
            index: dash_byte_range(self.index?)?,
            kind,
        };
        Some((mime_type, rep))
    }
}

fn build_sets<'a>(video: &'a [VideoStream], audio: &'a [AudioStream]) -> Vec<Set<'a>> {
    let mut sets: BTreeMap<SetKey, Set<'a>> = BTreeMap::new();

    for s in video {
        let common = Common {
            itag: s.itag,
            bitrate: s.bitrate,
            mime: &s.mime,
            url: &s.url,
            init: s.init_range.as_ref(),
            index: s.index_range.as_ref(),
            drm: s.drm,
        };
        let kind = RepKind::Video {
            width: s.width,
            height: s.height,
            fps: s.fps,
        };
        let Some((mime_type, rep)) = common.rep(common.itag.to_string(), kind) else {
            continue;
        };
        let (codec_rank, family) = codec_family(rep.codecs);
        let cicp = s.hdr.then(|| Cicp::from_codecs(rep.codecs));
        let key = SetKey {
            is_audio: false,
            not_default: false,
            track_id: String::new(),
            codec_rank,
            family,
            mime_type: mime_type.to_owned(),
            cicp,
        };
        sets.entry(key)
            .or_insert_with(|| Set {
                mime_type: mime_type.to_owned(),
                cicp,
                track: None,
                reps: Vec::new(),
            })
            .reps
            .push(rep);
    }

    for s in audio {
        let common = Common {
            itag: s.itag,
            bitrate: s.bitrate,
            mime: &s.mime,
            url: &s.url,
            init: s.init_range.as_ref(),
            index: s.index_range.as_ref(),
            drm: s.drm,
        };
        let id = match &s.track {
            Some(t) => format!("{}-{}", s.itag, t.id),
            None => s.itag.to_string(),
        };
        let kind = RepKind::Audio {
            channels: s.channels,
            sample_rate: s.sample_rate,
        };
        let Some((mime_type, rep)) = common.rep(id, kind) else {
            continue;
        };
        let (codec_rank, family) = codec_family(rep.codecs);
        let key = SetKey {
            is_audio: true,
            not_default: !s.track.as_ref().is_none_or(|t| t.is_default),
            track_id: s.track.as_ref().map(|t| t.id.clone()).unwrap_or_default(),
            codec_rank,
            family,
            mime_type: mime_type.to_owned(),
            cicp: None,
        };
        sets.entry(key)
            .or_insert_with(|| Set {
                mime_type: mime_type.to_owned(),
                cicp: None,
                track: s.track.as_ref(),
                reps: Vec::new(),
            })
            .reps
            .push(rep);
    }

    let mut sets: Vec<Set<'a>> = sets.into_values().collect();
    let mut seen = HashSet::new();
    for set in &mut sets {
        set.reps
            .sort_by(|a, b| (a.bandwidth, &a.id).cmp(&(b.bandwidth, &b.id)));
        // Duplicate itags occur, e.g. DRC audio variants share the itag of the original.
        for rep in &mut set.reps {
            let base = rep.id.clone();
            let mut n = 2;
            while !seen.insert(rep.id.clone()) {
                rep.id = format!("{base}-{n}");
                n += 1;
            }
        }
    }
    sets
}

fn role(track: &AudioTrack) -> &'static str {
    match track.track_type {
        Some(AudioTrackType::Original) => "main",
        Some(AudioTrackType::Dubbed | AudioTrackType::DubbedAuto) => "dub",
        Some(AudioTrackType::Descriptive) => "description",
        None if track.is_default => "main",
        None => "alternate",
    }
}

fn track_lang(track: &AudioTrack) -> Option<&str> {
    track
        .lang
        .as_deref()
        .or_else(|| track.id.split('.').next())
        .filter(|l| !l.is_empty())
}

fn write_set<W: io::Write>(w: &mut Writer<W>, id: usize, set: &Set<'_>) -> io::Result<()> {
    let id = id.to_string();
    let content_type = if set.mime_type.starts_with("audio/") {
        "audio"
    } else {
        "video"
    };
    let mut el = w.create_element("AdaptationSet").with_attributes([
        ("id", id.as_str()),
        ("contentType", content_type),
        ("mimeType", set.mime_type.as_str()),
        ("subsegmentAlignment", "true"),
        ("subsegmentStartsWithSAP", "1"),
        ("startWithSAP", "1"),
    ]);
    if let Some(track) = set.track {
        if let Some(lang) = track_lang(track) {
            el = el.with_attribute(("lang", lang));
        }
        if track.is_default {
            // dash.js picks the initial track by highest selectionPriority.
            el = el.with_attribute(("selectionPriority", "2"));
        }
    }
    el.write_inner_content(|w| {
        // Transfer is essential so SDR-only clients skip PQ/HLG sets (DASH-IF IOP).
        if let Some(cicp) = set.cicp {
            descriptor(
                w,
                "EssentialProperty",
                CICP_TRANSFER,
                &cicp.transfer.to_string(),
            )?;
            descriptor(
                w,
                "SupplementalProperty",
                CICP_PRIMARIES,
                &cicp.primaries.to_string(),
            )?;
            descriptor(
                w,
                "SupplementalProperty",
                CICP_MATRIX,
                &cicp.matrix.to_string(),
            )?;
        }
        if let Some(track) = set.track {
            w.create_element("Label")
                .write_text_content(BytesText::new(&track.lang_name))?;
            descriptor(w, "Role", ROLE_SCHEME, role(track))?;
            // The DASH role scheme has no value for machine dubbing.
            if track.track_type == Some(AudioTrackType::DubbedAuto) {
                descriptor(w, "Role", OXISTREAM_ROLE_SCHEME, "auto-dub")?;
            }
        }
        for rep in &set.reps {
            write_rep(w, rep)?;
        }
        Ok(())
    })?;
    Ok(())
}

fn write_rep<W: io::Write>(w: &mut Writer<W>, rep: &Rep<'_>) -> io::Result<()> {
    let bandwidth = rep.bandwidth.to_string();
    let mut el = w.create_element("Representation").with_attributes([
        ("id", rep.id.as_str()),
        ("codecs", rep.codecs),
        ("bandwidth", bandwidth.as_str()),
    ]);
    let mut channels = None;
    match rep.kind {
        RepKind::Video { width, height, fps } => {
            el = el.with_attributes([
                ("width", width.to_string().as_str()),
                ("height", height.to_string().as_str()),
                ("frameRate", fps.to_string().as_str()),
            ]);
        }
        RepKind::Audio {
            channels: ch,
            sample_rate,
        } => {
            if let Some(rate) = sample_rate {
                el = el.with_attribute(("audioSamplingRate", rate.to_string().as_str()));
            }
            channels = ch;
        }
    }
    el.write_inner_content(|w| {
        if let Some(ch) = channels {
            descriptor(
                w,
                "AudioChannelConfiguration",
                CHANNEL_SCHEME,
                &ch.to_string(),
            )?;
        }
        w.create_element("BaseURL")
            .write_text_content(BytesText::new(rep.url))?;
        w.create_element("SegmentBase")
            .with_attribute(("indexRange", rep.index.as_str()))
            .write_inner_content(|w| {
                w.create_element("Initialization")
                    .with_attribute(("range", rep.init.as_str()))
                    .write_empty()
                    .map(|_| ())
            })?;
        Ok(())
    })?;
    Ok(())
}

fn descriptor<W: io::Write>(
    w: &mut Writer<W>,
    name: &str,
    scheme: &str,
    value: &str,
) -> io::Result<()> {
    w.create_element(name)
        .with_attributes([("schemeIdUri", scheme), ("value", value)])
        .write_empty()?;
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn byte_range_keeps_inclusive_end() {
        assert_eq!(dash_byte_range(&(0..740)).as_deref(), Some("0-740"));
        assert_eq!(dash_byte_range(&(741..1204)).as_deref(), Some("741-1204"));
        assert_eq!(dash_byte_range(&(5..5)).as_deref(), Some("5-5"));
        assert_eq!(dash_byte_range(&Range { start: 10, end: 9 }), None);
    }

    #[test]
    fn formats_iso_duration() {
        assert_eq!(iso_duration(Duration::from_secs(212)), "PT212S");
        assert_eq!(iso_duration(Duration::from_millis(212_091)), "PT212.091S");
        assert_eq!(iso_duration(Duration::from_millis(5)), "PT0.005S");
    }

    #[test]
    fn parses_mime() {
        assert_eq!(
            parse_mime(r#"video/mp4; codecs="avc1.640028""#),
            Some(("video/mp4", "avc1.640028"))
        );
        assert_eq!(parse_mime("video/mp4"), None);
        assert_eq!(parse_mime(r#"audio/webm; codecs="""#), None);
    }

    #[test]
    fn reads_cicp_from_codecs() {
        assert_eq!(
            Cicp::from_codecs("vp09.02.51.10.01.09.18.09.00"),
            Cicp {
                primaries: 9,
                transfer: 18,
                matrix: 9
            }
        );
        assert_eq!(
            Cicp::from_codecs("av01.0.13M.10.0.110.09.16.09.0"),
            Cicp::HDR10
        );
        assert_eq!(Cicp::from_codecs("vp9"), Cicp::HDR10);
    }
}
