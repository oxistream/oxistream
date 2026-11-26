use std::time::Duration;

use oxistream_dash::{AudioStream, AudioTrack, AudioTrackType, Error, VideoStream, generate};
use quick_xml::Reader;
use quick_xml::events::Event;

const DURATION: Duration = Duration::from_millis(212_091);

fn url(itag: u32, extra: &str) -> String {
    format!("https://oxistream.example/proxy/video/abc{itag}?sig=x{extra}")
}

fn container(codecs: &str) -> &'static str {
    if codecs.starts_with("vp") || codecs == "opus" {
        "webm"
    } else {
        "mp4"
    }
}

/// Init range is `0-init_end`, index range follows directly up to `index_end` (inclusive).
fn video(
    itag: u32,
    codecs: &str,
    bitrate: u32,
    height: u32,
    fps: u8,
    init_end: u32,
    index_end: u32,
) -> VideoStream {
    VideoStream {
        url: url(itag, ""),
        itag,
        bitrate,
        init_range: Some(0..init_end),
        index_range: Some(init_end + 1..index_end),
        width: match height {
            240 => 426,
            480 => 854,
            h => h * 16 / 9,
        },
        height,
        fps,
        hdr: false,
        mime: format!(r#"video/{}; codecs="{codecs}""#, container(codecs)),
        drm: false,
    }
}

fn hdr(mut stream: VideoStream) -> VideoStream {
    stream.hdr = true;
    stream
}

fn audio(itag: u32, codecs: &str, bitrate: u32, init_end: u32, index_end: u32) -> AudioStream {
    AudioStream {
        url: url(itag, ""),
        itag,
        bitrate,
        init_range: Some(0..init_end),
        index_range: Some(init_end + 1..index_end),
        mime: format!(r#"audio/{}; codecs="{codecs}""#, container(codecs)),
        channels: Some(2),
        sample_rate: Some(if codecs == "opus" { 48_000 } else { 44_100 }),
        track: None,
        drm: false,
    }
}

fn typical_video() -> Vec<VideoStream> {
    vec![
        video(160, "avc1.4d400c", 113_000, 144, 30, 737, 1229),
        video(133, "avc1.4d4015", 248_000, 240, 30, 738, 1230),
        video(134, "avc1.4d401e", 553_000, 360, 30, 738, 1230),
        video(135, "avc1.4d401f", 1_155_000, 480, 30, 738, 1230),
        video(136, "avc1.4d401f", 2_310_000, 720, 30, 738, 1230),
        video(137, "avc1.640028", 4_350_000, 1080, 30, 739, 1231),
        video(278, "vp9", 95_000, 144, 30, 218, 936),
        video(242, "vp9", 224_000, 240, 30, 219, 937),
        video(243, "vp9", 405_000, 360, 30, 219, 938),
        video(244, "vp9", 750_000, 480, 30, 219, 938),
        video(247, "vp9", 1_505_000, 720, 30, 219, 939),
        video(248, "vp9", 2_646_000, 1080, 30, 219, 940),
        video(394, "av01.0.00M.08", 83_000, 144, 30, 699, 1191),
        video(398, "av01.0.05M.08", 1_135_000, 720, 30, 699, 1191),
        video(399, "av01.0.08M.08", 2_105_000, 1080, 30, 699, 1191),
    ]
}

fn typical_audio() -> Vec<AudioStream> {
    vec![
        audio(140, "mp4a.40.2", 130_000, 631, 1135),
        audio(139, "mp4a.40.5", 49_000, 640, 1144),
        audio(249, "opus", 54_000, 258, 882),
        audio(250, "opus", 71_000, 258, 882),
        audio(251, "opus", 141_000, 258, 882),
    ]
}

fn track(id: &str, lang: &str, name: &str, is_default: bool, kind: AudioTrackType) -> AudioTrack {
    AudioTrack {
        id: id.to_owned(),
        lang: Some(lang.to_owned()),
        lang_name: name.to_owned(),
        is_default,
        track_type: Some(kind),
    }
}

fn assert_well_formed(xml: &str) {
    let mut reader = Reader::from_str(xml);
    let mut depth = 0i32;
    loop {
        match reader.read_event().expect("well-formed xml") {
            Event::Start(_) => depth += 1,
            Event::End(_) => depth -= 1,
            Event::Eof => break,
            _ => {}
        }
    }
    assert_eq!(depth, 0);
}

fn mpd(video: &[VideoStream], audio: &[AudioStream]) -> String {
    let xml = generate(DURATION, video, audio).expect("mpd");
    assert_well_formed(&xml);
    xml
}

#[test]
fn typical() {
    insta::assert_snapshot!(mpd(&typical_video(), &typical_audio()));
}

#[test]
fn input_order_does_not_matter() {
    let (mut v, mut a) = (typical_video(), typical_audio());
    let expected = mpd(&v, &a);
    v.reverse();
    a.reverse();
    assert_eq!(mpd(&v, &a), expected);
}

#[test]
fn multi_audio_tracks() {
    use AudioTrackType::{Descriptive, Dubbed, DubbedAuto, Original};
    let tracks = [
        track("en.4", "en", "English original", false, Original),
        track("de-DE.3", "de-DE", "German", true, Dubbed),
        track(
            "es-US.3",
            "es-US",
            "Spanish (Latin America)",
            false,
            DubbedAuto,
        ),
        track(
            "en-US.2",
            "en-US",
            "English descriptive",
            false,
            Descriptive,
        ),
    ];
    let audio: Vec<AudioStream> = tracks
        .iter()
        .flat_map(|t| {
            [
                audio(140, "mp4a.40.2", 130_000, 631, 1135),
                audio(251, "opus", 141_000, 258, 882),
            ]
            .map(|mut s| {
                s.url = url(s.itag, &format!("&xtags=lang%3D{}", t.id));
                s.track = Some(t.clone());
                s
            })
        })
        .collect();
    let video = [video(137, "avc1.640028", 4_350_000, 1080, 30, 739, 1231)];
    insta::assert_snapshot!(mpd(&video, &audio));
}

#[test]
fn hdr_streams() {
    let video = [
        video(248, "vp9", 2_646_000, 1080, 60, 219, 940),
        hdr(video(
            337,
            "vp09.02.51.10.01.09.16.09.00",
            21_000_000,
            2160,
            60,
            218,
            1143,
        )),
        hdr(video(
            336,
            "vp09.02.41.10.01.09.16.09.00",
            8_400_000,
            1440,
            60,
            218,
            1143,
        )),
        hdr(video(
            701,
            "av01.0.13M.10.0.110.09.16.09.0",
            15_000_000,
            2160,
            60,
            699,
            1251,
        )),
    ];
    let audio = [audio(251, "opus", 141_000, 258, 882)];
    insta::assert_snapshot!(mpd(&video, &audio));
}

#[test]
fn skips_unplayable_streams() {
    let mut otf = video(137, "avc1.640028", 4_350_000, 1080, 30, 739, 1231);
    otf.index_range = None;
    let mut no_codecs = video(248, "vp9", 2_646_000, 1080, 30, 219, 940);
    no_codecs.mime = "video/webm".to_owned();
    let ok = video(136, "avc1.4d401f", 2_310_000, 720, 30, 738, 1230);
    let mut no_init = audio(140, "mp4a.40.2", 130_000, 631, 1135);
    no_init.init_range = None;
    let mut drm = audio(251, "opus", 141_000, 258, 882);
    drm.drm = true;

    insta::assert_snapshot!(mpd(&[otf, no_codecs, ok], &[no_init, drm]));
}

#[test]
fn all_skipped_is_error() {
    let mut v = typical_video();
    for s in &mut v {
        s.init_range = None;
    }
    let mut a = typical_audio();
    for s in &mut a {
        s.drm = true;
    }
    assert!(matches!(
        generate(DURATION, &v, &a),
        Err(Error::NoPlayableStreams)
    ));
    assert!(matches!(
        generate(DURATION, &[], &[]),
        Err(Error::NoPlayableStreams)
    ));
}

#[test]
fn duplicate_itags_get_unique_ids() {
    // DRC variants share itag and track with the regular stream.
    let a = [
        audio(251, "opus", 141_000, 258, 882),
        audio(251, "opus", 139_000, 258, 880),
    ];
    let xml = mpd(&[], &a);
    assert!(xml.contains(r#"id="251""#));
    assert!(xml.contains(r#"id="251-2""#));
}

#[test]
fn escapes_urls_and_labels() {
    let mut v = video(137, "avc1.640028", 4_350_000, 1080, 30, 739, 1231);
    v.url = "https://oxistream.example/proxy/video/x?a=1&b=<2>&c=\"3\"".to_owned();
    let mut a = audio(140, "mp4a.40.2", 130_000, 631, 1135);
    let name = "Français <dub> & \"more\"";
    a.track = Some(track("fr.3", "fr", name, true, AudioTrackType::Dubbed));
    let xml = mpd(&[v], &[a]);
    assert!(xml.contains("a=1&amp;b=&lt;2&gt;"));
    insta::assert_snapshot!(xml);
}
