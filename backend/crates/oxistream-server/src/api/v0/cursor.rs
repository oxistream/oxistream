//! Continuation tokens for keyset pagination by timestamp and video ID.

use rustypipe::validate;
use time::OffsetDateTime;

pub fn encode(at: OffsetDateTime, video_id: &str) -> String {
    let micros = at.unix_timestamp_nanos() / 1000;
    format!("{micros}.{video_id}")
}

pub fn decode(token: &str) -> Option<(OffsetDateTime, String)> {
    let (micros, video_id) = token.split_once('.')?;
    let micros: i128 = micros.parse().ok()?;
    let at = OffsetDateTime::from_unix_timestamp_nanos(micros.checked_mul(1000)?).ok()?;
    validate::video_id(video_id).ok()?;
    Some((at, video_id.to_owned()))
}

#[cfg(test)]
mod tests {
    use time::macros::datetime;

    use super::*;

    #[test]
    fn round_trips() {
        let at = datetime!(2026-11-26 12:34:56.123456 UTC);
        let token = encode(at, "dQw4w9WgXcQ");
        assert_eq!(decode(&token), Some((at, "dQw4w9WgXcQ".to_owned())));
    }

    #[test]
    fn rejects_malformed_tokens() {
        for token in [
            "",
            "x",
            "12",
            "abc.dQw4w9WgXcQ",
            "12.not-a-video-id!",
            "1e99.dQw4w9WgXcQ",
        ] {
            assert_eq!(decode(token), None, "{token}");
        }
    }
}
