use std::time::Duration;

use base64::Engine;
use base64::engine::general_purpose::URL_SAFE_NO_PAD;
use hmac::{Hmac, KeyInit, Mac};
use sha2::Sha256;

/// Truncated HMAC-SHA256 tag length; 128 bits keeps URLs short.
const TAG_LEN: usize = 16;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Kind {
    Stream,
    Image,
    Subtitle,
}

impl Kind {
    pub fn as_str(self) -> &'static str {
        match self {
            Self::Stream => "stream",
            Self::Image => "image",
            Self::Subtitle => "subtitle",
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, thiserror::Error)]
pub enum SignError {
    #[error("invalid signature")]
    Invalid,
    #[error("expired")]
    Expired,
}

/// Signs proxy URLs statelessly so every replica sharing the key can serve them.
#[derive(Clone)]
pub struct Signer {
    mac: Hmac<Sha256>,
}

impl Signer {
    pub fn new(secret: &[u8]) -> Self {
        Self {
            mac: Hmac::new_from_slice(secret).expect("HMAC accepts keys of any length"),
        }
    }

    fn mac(&self, kind: Kind, payload: &str, exp: u64) -> Hmac<Sha256> {
        let mut mac = self.mac.clone();
        mac.update(kind.as_str().as_bytes());
        mac.update(b"\0");
        mac.update(payload.as_bytes());
        mac.update(b"\0");
        mac.update(exp.to_string().as_bytes());
        mac
    }

    pub fn sign(&self, kind: Kind, payload: &str, exp: u64) -> String {
        let tag = self.mac(kind, payload, exp).finalize().into_bytes();
        URL_SAFE_NO_PAD.encode(&tag[..TAG_LEN])
    }

    /// Checks `sig` in constant time, then the expiry against `now` (Unix seconds).
    pub fn verify(
        &self,
        kind: Kind,
        payload: &str,
        exp: u64,
        sig: &str,
        now: u64,
    ) -> Result<(), SignError> {
        let tag = URL_SAFE_NO_PAD
            .decode(sig)
            .map_err(|_| SignError::Invalid)?;
        if tag.len() != TAG_LEN {
            return Err(SignError::Invalid);
        }
        self.mac(kind, payload, exp)
            .verify_truncated_left(&tag)
            .map_err(|_| SignError::Invalid)?;
        if exp <= now {
            return Err(SignError::Expired);
        }
        Ok(())
    }
}

/// `now + ttl`, rounded up to a multiple of `bucket` so URLs stay stable for a while.
pub fn expiry(now: u64, ttl: Duration, bucket: Duration) -> u64 {
    let bucket = bucket.as_secs().max(1);
    (now + ttl.as_secs()).div_ceil(bucket) * bucket
}

#[cfg(test)]
mod tests {
    use super::*;

    const NOW: u64 = 1_800_000_000;

    fn signer() -> Signer {
        Signer::new(b"0123456789abcdef0123456789abcdef")
    }

    #[test]
    fn accepts_valid_signature() {
        let s = signer();
        let sig = s.sign(Kind::Stream, "dQw4w9WgXcQ/251/", NOW + 60);
        assert_eq!(sig.len(), 22);
        assert_eq!(
            s.verify(Kind::Stream, "dQw4w9WgXcQ/251/", NOW + 60, &sig, NOW),
            Ok(())
        );
    }

    #[test]
    fn rejects_tampering() {
        let s = signer();
        let sig = s.sign(Kind::Stream, "dQw4w9WgXcQ/251/", NOW + 60);
        let verify =
            |payload: &str, exp: u64, sig: &str| s.verify(Kind::Stream, payload, exp, sig, NOW);
        assert_eq!(
            verify("dQw4w9WgXcQ/140/", NOW + 60, &sig),
            Err(SignError::Invalid)
        );
        assert_eq!(
            verify("dQw4w9WgXcQ/251/", NOW + 61, &sig),
            Err(SignError::Invalid)
        );
        assert_eq!(
            verify("dQw4w9WgXcQ/251/", NOW + 60, "AAAA"),
            Err(SignError::Invalid)
        );
        assert_eq!(
            verify("dQw4w9WgXcQ/251/", NOW + 60, "not base64!"),
            Err(SignError::Invalid)
        );

        let mut flipped = sig.into_bytes();
        flipped[0] = if flipped[0] == b'A' { b'B' } else { b'A' };
        let flipped = String::from_utf8(flipped).unwrap();
        assert_eq!(
            verify("dQw4w9WgXcQ/251/", NOW + 60, &flipped),
            Err(SignError::Invalid)
        );
    }

    #[test]
    fn rejects_other_kind_and_key() {
        let s = signer();
        let sig = s.sign(Kind::Image, "payload", NOW + 60);
        assert_eq!(
            s.verify(Kind::Subtitle, "payload", NOW + 60, &sig, NOW),
            Err(SignError::Invalid)
        );
        let other = Signer::new(b"another-secret-another-secret-xx");
        assert_eq!(
            other.verify(Kind::Image, "payload", NOW + 60, &sig, NOW),
            Err(SignError::Invalid)
        );
    }

    #[test]
    fn rejects_expired() {
        let s = signer();
        let sig = s.sign(Kind::Image, "payload", NOW);
        assert_eq!(
            s.verify(Kind::Image, "payload", NOW, &sig, NOW),
            Err(SignError::Expired)
        );
    }

    #[test]
    fn expiry_rounds_up_to_bucket() {
        let day = Duration::from_hours(24);
        let week = Duration::from_hours(7 * 24);
        let exp = expiry(NOW, week, day);
        assert_eq!(exp % 86_400, 0);
        assert!(exp >= NOW + week.as_secs() && exp < NOW + week.as_secs() + 86_400);
        assert_eq!(expiry(NOW + 1, week, day), exp);
        assert_eq!(expiry(100, Duration::from_secs(20), Duration::ZERO), 120);
    }
}
