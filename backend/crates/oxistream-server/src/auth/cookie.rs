//! Session tokens and the session cookie.

use std::time::Duration;

use axum::http::{HeaderMap, HeaderValue, header};
use base64::Engine;
use base64::engine::general_purpose::URL_SAFE_NO_PAD;
use oxistream_db::TokenHash;
use sha2::{Digest, Sha256};

pub const SESSION_COOKIE: &str = "oxistream_session";

const TOKEN_BYTES: usize = 32;

/// A new random 256-bit session token and its hash for storage.
pub fn new_token() -> (String, TokenHash) {
    let mut bytes = [0u8; TOKEN_BYTES];
    getrandom::fill(&mut bytes).expect("system random number generator");
    (URL_SAFE_NO_PAD.encode(bytes), Sha256::digest(bytes).into())
}

/// Hash of a token from a cookie; `None` if it is not a well-formed token.
pub fn hash_token(token: &str) -> Option<TokenHash> {
    let bytes = URL_SAFE_NO_PAD.decode(token).ok()?;
    (bytes.len() == TOKEN_BYTES).then(|| Sha256::digest(&bytes).into())
}

/// The session token from the request's `Cookie` headers.
pub fn session_token(headers: &HeaderMap) -> Option<&str> {
    headers
        .get_all(header::COOKIE)
        .iter()
        .filter_map(|value| value.to_str().ok())
        .flat_map(|value| value.split(';'))
        .find_map(|pair| {
            let (name, value) = pair.trim().split_once('=')?;
            (name == SESSION_COOKIE).then_some(value.trim_matches('"'))
        })
}

pub fn session_cookie(token: &str, max_age: Duration, secure: bool) -> HeaderValue {
    build(token, max_age.as_secs(), secure)
}

pub fn clear_session_cookie(secure: bool) -> HeaderValue {
    build("", 0, secure)
}

fn build(value: &str, max_age: u64, secure: bool) -> HeaderValue {
    let secure = if secure { "; Secure" } else { "" };
    HeaderValue::try_from(format!(
        "{SESSION_COOKIE}={value}; HttpOnly; SameSite=Lax; Path=/; Max-Age={max_age}{secure}"
    ))
    .expect("cookie is a valid header value")
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn tokens_are_random_and_hash_consistently() {
        let (a, hash_a) = new_token();
        let (b, _) = new_token();
        assert_ne!(a, b);
        assert_eq!(a.len(), 43);
        assert_eq!(hash_token(&a), Some(hash_a));
        assert_eq!(hash_token("not-a-token"), None);
        assert_eq!(hash_token(&URL_SAFE_NO_PAD.encode([0u8; 16])), None);
    }

    #[test]
    fn builds_cookies() {
        assert_eq!(
            session_cookie("abc", Duration::from_hours(24), true),
            "oxistream_session=abc; HttpOnly; SameSite=Lax; Path=/; Max-Age=86400; Secure"
        );
        assert_eq!(
            clear_session_cookie(false),
            "oxistream_session=; HttpOnly; SameSite=Lax; Path=/; Max-Age=0"
        );
    }

    #[test]
    fn reads_token_from_cookie_headers() {
        let mut headers = HeaderMap::new();
        assert_eq!(session_token(&headers), None);
        headers.append(header::COOKIE, HeaderValue::from_static("sidebar=true"));
        headers.append(
            header::COOKIE,
            HeaderValue::from_static("a=1; oxistream_session=tok; b=2"),
        );
        assert_eq!(session_token(&headers), Some("tok"));
    }
}
