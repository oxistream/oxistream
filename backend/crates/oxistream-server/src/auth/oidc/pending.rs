//! The login in progress, kept in a short-lived HMAC-signed cookie so that any replica can
//! finish it. It holds nothing the browser must not see: the PKCE verifier is only useful
//! together with the authorization code, which the same browser receives anyway.

use std::time::{Duration, SystemTime, UNIX_EPOCH};

use axum::http::{HeaderMap, HeaderValue, header};
use base64::Engine;
use base64::engine::general_purpose::URL_SAFE_NO_PAD;
use hmac::{Hmac, KeyInit, Mac};
use serde::{Deserialize, Serialize};
use sha2::Sha256;

pub const COOKIE: &str = "oxistream_oidc";
const COOKIE_PATH: &str = "/api/v0/auth/oidc";
pub const TTL: Duration = Duration::from_mins(10);
const DOMAIN: &[u8] = b"oxistream-oidc-pending\0";

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct PendingLogin {
    pub state: String,
    pub nonce: String,
    pub pkce_verifier: String,
    /// Same-origin path to continue to.
    pub redirect: String,
    /// Unix seconds.
    pub exp: u64,
}

#[derive(Clone)]
pub struct PendingCodec {
    mac: Hmac<Sha256>,
}

impl PendingCodec {
    pub fn new(secret: &[u8]) -> Self {
        Self {
            mac: Hmac::new_from_slice(secret).expect("HMAC accepts keys of any length"),
        }
    }

    fn tag(&self, payload: &str) -> Hmac<Sha256> {
        let mut mac = self.mac.clone();
        mac.update(DOMAIN);
        mac.update(payload.as_bytes());
        mac
    }

    pub fn encode(&self, pending: &PendingLogin) -> String {
        let json = serde_json::to_vec(pending).expect("pending login serializes");
        let payload = URL_SAFE_NO_PAD.encode(json);
        let tag = URL_SAFE_NO_PAD.encode(self.tag(&payload).finalize().into_bytes());
        format!("{payload}.{tag}")
    }

    /// Verifies the signature in constant time and rejects expired values.
    pub fn decode(&self, value: &str, now: u64) -> Option<PendingLogin> {
        let (payload, tag) = value.split_once('.')?;
        let tag = URL_SAFE_NO_PAD.decode(tag).ok()?;
        self.tag(payload).verify_slice(&tag).ok()?;
        let pending: PendingLogin =
            serde_json::from_slice(&URL_SAFE_NO_PAD.decode(payload).ok()?).ok()?;
        (pending.exp > now).then_some(pending)
    }
}

pub fn now() -> u64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map_or(0, |d| d.as_secs())
}

pub fn cookie(value: &str, secure: bool) -> HeaderValue {
    build(value, TTL.as_secs(), secure)
}

pub fn clear_cookie(secure: bool) -> HeaderValue {
    build("", 0, secure)
}

fn build(value: &str, max_age: u64, secure: bool) -> HeaderValue {
    let secure = if secure { "; Secure" } else { "" };
    // Lax: the provider redirects back with a top-level GET navigation.
    HeaderValue::try_from(format!(
        "{COOKIE}={value}; HttpOnly; SameSite=Lax; Path={COOKIE_PATH}; Max-Age={max_age}{secure}"
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

    fn pending(exp: u64) -> PendingLogin {
        PendingLogin {
            state: "state".into(),
            nonce: "nonce".into(),
            pkce_verifier: "verifier".into(),
            redirect: "/watch?v=x".into(),
            exp,
        }
    }

    #[test]
    fn round_trips_and_rejects_tampering() {
        let codec = PendingCodec::new(b"0123456789abcdef0123456789abcdef");
        let value = codec.encode(&pending(100));
        assert_eq!(codec.decode(&value, 99), Some(pending(100)));
        assert_eq!(codec.decode(&value, 100), None, "expired");

        let other = PendingCodec::new(b"another-secret-another-secret-xx");
        assert_eq!(other.decode(&value, 0), None);

        let (payload, tag) = value.split_once('.').unwrap();
        let forged = URL_SAFE_NO_PAD.encode(
            serde_json::to_vec(&PendingLogin {
                redirect: "/evil".into(),
                ..pending(100)
            })
            .unwrap(),
        );
        assert_eq!(codec.decode(&format!("{forged}.{tag}"), 0), None);
        assert_eq!(codec.decode(payload, 0), None);
        assert_eq!(codec.decode("garbage", 0), None);
    }

    #[test]
    fn cookie_is_scoped_to_the_oidc_endpoints() {
        assert_eq!(
            cookie("v", true),
            "oxistream_oidc=v; HttpOnly; SameSite=Lax; Path=/api/v0/auth/oidc; Max-Age=600; Secure"
        );
        let mut headers = HeaderMap::new();
        headers.insert(
            header::COOKIE,
            HeaderValue::from_static("oxistream_session=a; oxistream_oidc=b"),
        );
        assert_eq!(from_headers(&headers), Some("b"));
    }
}
