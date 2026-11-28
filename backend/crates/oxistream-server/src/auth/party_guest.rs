//! Party guests when `OXISTREAM_REQUIRE_LOGIN` is set: a guest in a watch party gets an
//! HMAC-signed cookie naming the room and its participant key, so any replica can let it use
//! the app like an anonymous user while it is in that room.

use std::time::{Duration, SystemTime, UNIX_EPOCH};

use axum::http::{HeaderMap, HeaderValue, header};
use base64::Engine;
use base64::engine::general_purpose::URL_SAFE_NO_PAD;
use hmac::{Hmac, KeyInit, Mac};
use moka::future::Cache;
use oxistream_db::{Presence, RoomRepository};
use sha2::Sha256;

use crate::api::error::ApiError;

pub const COOKIE: &str = "oxistream_party";
const DOMAIN: &[u8] = b"oxistream-party-guest\0";
const TAG_LEN: usize = 16;
/// How long a guest keeps access after it was last confirmed in the room, so a reload or a
/// reconnect does not lock it out. Clients confirm every 30 s.
pub const GRACE: Duration = Duration::from_mins(2);
/// Confirmed presence is remembered this long, as proxy requests are frequent.
const CACHE_TTL: Duration = Duration::from_secs(5);
/// The browser may drop the cookie once the room would have expired.
const MAX_AGE: Duration = crate::rooms::EXPIRE_AFTER;

/// A guest's claim to be in `room` as participant `key`, confirmed at `seen` (Unix seconds).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PartyGuest {
    pub room: String,
    pub key: String,
    pub seen: u64,
}

pub struct PartyGuests {
    mac: Hmac<Sha256>,
    present: Cache<(String, String), ()>,
}

impl PartyGuests {
    pub fn new(secret: &[u8]) -> Self {
        Self {
            mac: Hmac::new_from_slice(secret).expect("HMAC accepts keys of any length"),
            present: Cache::builder()
                .max_capacity(10_000)
                .time_to_live(CACHE_TTL)
                .build(),
        }
    }

    fn tag(&self, payload: &str) -> Hmac<Sha256> {
        let mut mac = self.mac.clone();
        mac.update(DOMAIN);
        mac.update(payload.as_bytes());
        mac
    }

    pub fn encode(&self, guest: &PartyGuest) -> String {
        let payload = format!("{}.{}.{}", guest.room, guest.key, guest.seen);
        let tag = self.tag(&payload).finalize().into_bytes();
        format!("{payload}.{}", URL_SAFE_NO_PAD.encode(&tag[..TAG_LEN]))
    }

    /// Verifies the signature in constant time.
    pub fn decode(&self, value: &str) -> Option<PartyGuest> {
        let (payload, tag) = value.rsplit_once('.')?;
        let tag = URL_SAFE_NO_PAD.decode(tag).ok()?;
        if tag.len() != TAG_LEN {
            return None;
        }
        self.tag(payload).verify_truncated_left(&tag).ok()?;
        let mut parts = payload.split('.');
        let (room, key, seen) = (parts.next()?, parts.next()?, parts.next()?);
        if parts.next().is_some() {
            return None;
        }
        Some(PartyGuest {
            room: room.to_owned(),
            key: key.to_owned(),
            seen: seen.parse().ok()?,
        })
    }

    /// Whether the guest may use the app now: it is in the room, or was until moments ago and
    /// was neither removed nor is the room gone.
    pub async fn admits(
        &self,
        rooms: &dyn RoomRepository,
        guest: &PartyGuest,
        now: u64,
    ) -> Result<bool, ApiError> {
        let id = (guest.room.clone(), guest.key.clone());
        if self.present.contains_key(&id) {
            return Ok(true);
        }
        match rooms.presence(&guest.room, &guest.key).await? {
            Presence::Present => {
                self.present.insert(id, ()).await;
                Ok(true)
            }
            Presence::Absent => Ok(now.saturating_sub(guest.seen) < GRACE.as_secs()),
            Presence::Gone => Ok(false),
        }
    }

    /// Forgets confirmed presence, e.g. once the guest left.
    pub async fn forget(&self, room: &str, key: &str) {
        self.present
            .invalidate(&(room.to_owned(), key.to_owned()))
            .await;
    }
}

pub fn now() -> u64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map_or(0, |d| d.as_secs())
}

pub fn cookie(value: &str, secure: bool) -> HeaderValue {
    build(value, MAX_AGE.as_secs(), secure)
}

pub fn clear_cookie(secure: bool) -> HeaderValue {
    build("", 0, secure)
}

fn build(value: &str, max_age: u64, secure: bool) -> HeaderValue {
    let secure = if secure { "; Secure" } else { "" };
    HeaderValue::try_from(format!(
        "{COOKIE}={value}; HttpOnly; SameSite=Lax; Path=/; Max-Age={max_age}{secure}"
    ))
    .expect("cookie is a valid header value")
}

pub fn from_headers(headers: &HeaderMap) -> Option<&str> {
    headers
        .get_all(header::COOKIE)
        .iter()
        .filter_map(|value| value.to_str().ok())
        .flat_map(|value| value.split(';'))
        .find_map(|pair| {
            let (name, value) = pair.trim().split_once('=')?;
            (name == COOKIE).then_some(value.trim_matches('"'))
        })
}

#[cfg(test)]
mod tests {
    use super::*;

    fn guest() -> PartyGuest {
        PartyGuest {
            room: "rrrrrrrrrrrrrrrrrrrrrA".into(),
            key: "g:abc".into(),
            seen: 100,
        }
    }

    #[test]
    fn round_trips_and_rejects_tampering() {
        let guests = PartyGuests::new(b"0123456789abcdef0123456789abcdef");
        let value = guests.encode(&guest());
        assert_eq!(guests.decode(&value), Some(guest()));

        let other = PartyGuests::new(b"another-secret-another-secret-xx");
        assert_eq!(other.decode(&value), None);

        let (_, tag) = value.rsplit_once('.').unwrap();
        let forged = format!("rrrrrrrrrrrrrrrrrrrrrA.g:abc.999.{tag}");
        assert_eq!(guests.decode(&forged), None);
        assert_eq!(guests.decode("garbage"), None);
        assert_eq!(guests.decode(""), None);
    }

    #[test]
    fn cookie_is_http_only() {
        assert_eq!(
            cookie("v", true),
            "oxistream_party=v; HttpOnly; SameSite=Lax; Path=/; Max-Age=86400; Secure"
        );
        assert_eq!(
            clear_cookie(false),
            "oxistream_party=; HttpOnly; SameSite=Lax; Path=/; Max-Age=0"
        );
    }
}
