use std::ops::Range;

/// A video-only adaptive stream, modelled after `rustypipe::model::VideoStream`.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct VideoStream {
    /// Final (proxied) URL written to `BaseURL`.
    pub url: String,
    pub itag: u32,
    /// Peak bitrate in bits/second.
    pub bitrate: u32,
    /// Byte range as reported by YouTube (`end` is inclusive).
    pub init_range: Option<Range<u32>>,
    /// Byte range as reported by YouTube (`end` is inclusive).
    pub index_range: Option<Range<u32>>,
    pub width: u32,
    pub height: u32,
    pub fps: u8,
    pub hdr: bool,
    /// Full MIME type including codecs, e.g. `video/mp4; codecs="avc1.640028"`.
    pub mime: String,
    /// True if the stream is DRM-protected.
    pub drm: bool,
}

/// An audio-only adaptive stream, modelled after `rustypipe::model::AudioStream`.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct AudioStream {
    /// Final (proxied) URL written to `BaseURL`.
    pub url: String,
    pub itag: u32,
    /// Peak bitrate in bits/second.
    pub bitrate: u32,
    /// Byte range as reported by YouTube (`end` is inclusive).
    pub init_range: Option<Range<u32>>,
    /// Byte range as reported by YouTube (`end` is inclusive).
    pub index_range: Option<Range<u32>>,
    /// Full MIME type including codecs, e.g. `audio/webm; codecs="opus"`.
    pub mime: String,
    pub channels: Option<u8>,
    /// Sampling rate in Hz, if known.
    pub sample_rate: Option<u32>,
    /// Audio track the stream belongs to; `None` if the video has a single track.
    pub track: Option<AudioTrack>,
    /// True if the stream is DRM-protected.
    pub drm: bool,
}

/// Audio track metadata, modelled after `rustypipe::model::AudioTrack`.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct AudioTrack {
    /// Track ID, e.g. `en.4`.
    pub id: String,
    /// Language code, e.g. `en-US`.
    pub lang: Option<String>,
    /// Human-readable language name, e.g. "English (United States) original".
    pub lang_name: String,
    /// True if YouTube selects this track by default.
    pub is_default: bool,
    pub track_type: Option<AudioTrackType>,
}

/// Audio track type, modelled after `rustypipe::model::AudioTrackType`.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum AudioTrackType {
    Original,
    Dubbed,
    DubbedAuto,
    Descriptive,
}
