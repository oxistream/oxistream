//! Validation of back-channel logout tokens (`OpenID` Connect Back-Channel Logout 1.0).

use base64::Engine;
use base64::engine::general_purpose::URL_SAFE_NO_PAD;
use openidconnect::core::{CoreJsonWebKey, CoreJsonWebKeySet, CoreJwsSigningAlgorithm};
use openidconnect::{JsonWebKey, JsonWebKeyUse, JwsSigningAlgorithm};
use serde::Deserialize;
use serde_json::Value;

use super::Oidc;

pub const LOGOUT_EVENT: &str = "http://schemas.openid.net/event/backchannel-logout";
/// Clock skew tolerated for `iat` and `exp`.
pub const LEEWAY_SECS: u64 = 60;
/// Tokens issued longer ago than this are rejected; their `jti` is kept at least as long.
pub const MAX_AGE_SECS: u64 = 10 * 60;

/// Who to sign out.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Logout {
    pub sid: Option<String>,
    pub sub: Option<String>,
    pub jti: String,
    /// Unix seconds until which the token could be replayed.
    pub replayable_until: u64,
}

#[derive(Debug, Clone, PartialEq, Eq, thiserror::Error)]
#[error("{0}")]
pub struct InvalidToken(pub &'static str);

#[derive(Deserialize)]
struct Header {
    alg: String,
    kid: Option<String>,
    typ: Option<String>,
}

#[derive(Deserialize)]
struct Claims {
    iss: Option<String>,
    aud: Option<Value>,
    iat: Option<u64>,
    exp: Option<u64>,
    jti: Option<String>,
    sid: Option<String>,
    sub: Option<String>,
    events: Option<serde_json::Map<String, Value>>,
    nonce: Option<Value>,
}

/// What a logout token is checked against.
pub struct Expected<'a> {
    pub issuer: &'a str,
    pub client_id: &'a str,
    /// Allowed signature algorithms, from discovery.
    pub algs: &'a [CoreJwsSigningAlgorithm],
    /// For `HS*` tokens signed with the client secret.
    pub client_secret: Option<&'a str>,
    pub now: u64,
}

struct Parsed<'a> {
    header: Header,
    alg: CoreJwsSigningAlgorithm,
    signing_input: &'a str,
    signature: Vec<u8>,
    payload: Vec<u8>,
}

fn decode<T: serde::de::DeserializeOwned>(part: &str) -> Option<T> {
    serde_json::from_slice(&URL_SAFE_NO_PAD.decode(part).ok()?).ok()
}

fn parse<'a>(token: &'a str, expected: &Expected<'_>) -> Result<Parsed<'a>, InvalidToken> {
    let malformed = InvalidToken("malformed token");
    let mut parts = token.split('.');
    let (Some(header), Some(payload), Some(signature), None) =
        (parts.next(), parts.next(), parts.next(), parts.next())
    else {
        return Err(malformed);
    };
    let header: Header = decode(header).ok_or(malformed.clone())?;
    if let Some(typ) = &header.typ {
        let typ = typ.to_ascii_lowercase();
        if typ != "logout+jwt" && typ != "application/logout+jwt" {
            return Err(InvalidToken("unexpected typ header"));
        }
    }
    let alg: CoreJwsSigningAlgorithm = serde_json::from_value(Value::String(header.alg.clone()))
        .map_err(|_| InvalidToken("unsupported algorithm"))?;
    if matches!(alg, CoreJwsSigningAlgorithm::None) || !expected.algs.contains(&alg) {
        return Err(InvalidToken("algorithm not allowed"));
    }
    let signing_input = &token[..header_len(token)];
    Ok(Parsed {
        header,
        alg,
        signing_input,
        signature: URL_SAFE_NO_PAD
            .decode(signature)
            .map_err(|_| malformed.clone())?,
        payload: URL_SAFE_NO_PAD.decode(payload).map_err(|_| malformed)?,
    })
}

/// Length of `header.payload`, the signed part of a compact JWS.
fn header_len(token: &str) -> usize {
    token.rfind('.').unwrap_or(token.len())
}

/// `None` if no key in `jwks` has the token's key ID (or, without one, fits its algorithm).
fn verify_signature(
    parsed: &Parsed<'_>,
    jwks: &CoreJsonWebKeySet,
    client_secret: Option<&str>,
) -> Option<bool> {
    let verify = |key: &CoreJsonWebKey| {
        key.verify_signature(
            &parsed.alg,
            parsed.signing_input.as_bytes(),
            &parsed.signature,
        )
        .is_ok()
    };
    if parsed.alg.uses_shared_secret() {
        let secret = client_secret?;
        return Some(verify(&CoreJsonWebKey::new_symmetric(
            secret.as_bytes().to_vec(),
        )));
    }
    let candidates: Vec<&CoreJsonWebKey> = jwks
        .keys()
        .iter()
        .filter(|key| key.key_use().is_none_or(JsonWebKeyUse::allows_signature))
        .filter(|key| match &parsed.header.kid {
            Some(kid) => key.key_id().is_some_and(|id| **id == *kid),
            None => true,
        })
        .collect();
    if candidates.is_empty() {
        return None;
    }
    Some(candidates.into_iter().any(verify))
}

fn check_claims(payload: &[u8], expected: &Expected<'_>) -> Result<Logout, InvalidToken> {
    let claims: Claims =
        serde_json::from_slice(payload).map_err(|_| InvalidToken("malformed claims"))?;
    if claims.iss.as_deref() != Some(expected.issuer) {
        return Err(InvalidToken("wrong issuer"));
    }
    let audience_ok = match &claims.aud {
        Some(Value::String(aud)) => aud == expected.client_id,
        Some(Value::Array(auds)) => auds.iter().any(|aud| aud == expected.client_id),
        _ => false,
    };
    if !audience_ok {
        return Err(InvalidToken("wrong audience"));
    }
    let iat = claims.iat.ok_or(InvalidToken("missing iat"))?;
    if iat > expected.now + LEEWAY_SECS {
        return Err(InvalidToken("issued in the future"));
    }
    if iat + MAX_AGE_SECS + LEEWAY_SECS < expected.now {
        return Err(InvalidToken("too old"));
    }
    if claims
        .exp
        .is_some_and(|exp| exp + LEEWAY_SECS <= expected.now)
    {
        return Err(InvalidToken("expired"));
    }
    if !claims
        .events
        .as_ref()
        .and_then(|events| events.get(LOGOUT_EVENT))
        .is_some_and(Value::is_object)
    {
        return Err(InvalidToken("missing logout event"));
    }
    if claims.nonce.is_some() {
        return Err(InvalidToken("nonce is not allowed"));
    }
    if claims.sid.is_none() && claims.sub.is_none() {
        return Err(InvalidToken("neither sid nor sub"));
    }
    let jti = claims
        .jti
        .filter(|jti| !jti.is_empty())
        .ok_or(InvalidToken("missing jti"))?;
    let replayable_until = (iat + MAX_AGE_SECS).max(claims.exp.unwrap_or(0)) + LEEWAY_SECS;
    Ok(Logout {
        sid: claims.sid,
        sub: claims.sub,
        jti,
        replayable_until,
    })
}

/// Checks the header, signature and claims of `token` against `jwks`; `Ok(None)` means no
/// key matched, so the caller may refresh the JWKS and try again.
pub fn validate_with(
    token: &str,
    jwks: &CoreJsonWebKeySet,
    expected: &Expected<'_>,
) -> Result<Option<Logout>, InvalidToken> {
    let parsed = parse(token, expected)?;
    match verify_signature(&parsed, jwks, expected.client_secret) {
        None => Ok(None),
        Some(false) => Err(InvalidToken("invalid signature")),
        Some(true) => check_claims(&parsed.payload, expected).map(Some),
    }
}

impl Oidc {
    /// Validates a logout token, refetching the provider's keys once on an unknown key ID.
    /// `Err(None)` means the provider has not been discovered yet.
    pub async fn validate_logout_token(
        &self,
        token: &str,
        now: u64,
    ) -> Result<Logout, Option<InvalidToken>> {
        let metadata = self.metadata().ok_or(None)?;
        let config = self.config();
        let algs = metadata.id_token_signing_alg_values_supported().clone();
        let expected = Expected {
            issuer: self.issuer(),
            client_id: &config.client_id,
            algs: &algs,
            client_secret: config.client_secret.as_deref(),
            now,
        };
        if let Some(logout) = validate_with(token, metadata.jwks(), &expected)? {
            return Ok(logout);
        }
        if self.refresh_jwks().await {
            let jwks = self.jwks().ok_or(None)?;
            if let Some(logout) = validate_with(token, &jwks, &expected)? {
                return Ok(logout);
            }
        }
        Err(Some(InvalidToken("no matching key")))
    }
}

#[cfg(test)]
mod tests {
    use openidconnect::PrivateSigningKey;
    use openidconnect::core::CoreHmacKey;
    use serde_json::json;

    use super::*;

    const NOW: u64 = 1_800_000_000;

    fn expected(algs: &[CoreJwsSigningAlgorithm]) -> Expected<'_> {
        Expected {
            issuer: "https://id.example.com",
            client_id: "oxistream",
            algs,
            client_secret: Some("client-secret-client-secret-0123"),
            now: NOW,
        }
    }

    fn claims() -> Value {
        json!({
            "iss": "https://id.example.com",
            "aud": "oxistream",
            "iat": NOW - 5,
            "jti": "jti-1",
            "sid": "sid-1",
            "events": { LOGOUT_EVENT: {} },
        })
    }

    fn hs256(header: &Value, claims: &Value) -> String {
        let input = format!(
            "{}.{}",
            URL_SAFE_NO_PAD.encode(header.to_string()),
            URL_SAFE_NO_PAD.encode(claims.to_string())
        );
        let key = CoreHmacKey::new(b"client-secret-client-secret-0123".to_vec());
        let signature = key
            .sign(&CoreJwsSigningAlgorithm::HmacSha256, input.as_bytes())
            .unwrap();
        format!("{input}.{}", URL_SAFE_NO_PAD.encode(signature))
    }

    fn check(header: &Value, claims: &Value) -> Result<Option<Logout>, InvalidToken> {
        let algs = [CoreJwsSigningAlgorithm::HmacSha256];
        validate_with(
            &hs256(header, claims),
            &CoreJsonWebKeySet::new(vec![]),
            &expected(&algs),
        )
    }

    fn err(claims: &Value) -> &'static str {
        check(&json!({ "alg": "HS256" }), claims).unwrap_err().0
    }

    #[test]
    fn accepts_valid_tokens() {
        let logout = check(&json!({ "alg": "HS256", "typ": "logout+jwt" }), &claims())
            .unwrap()
            .unwrap();
        assert_eq!(logout.sid.as_deref(), Some("sid-1"));
        assert_eq!(logout.jti, "jti-1");
        assert_eq!(
            logout.replayable_until,
            NOW - 5 + MAX_AGE_SECS + LEEWAY_SECS
        );

        let mut by_sub = claims();
        by_sub.as_object_mut().unwrap().remove("sid");
        by_sub["sub"] = json!("user-1");
        by_sub["aud"] = json!(["other", "oxistream"]);
        by_sub["exp"] = json!(NOW + 3600);
        let logout = check(&json!({ "alg": "HS256" }), &by_sub).unwrap().unwrap();
        assert_eq!(logout.sub.as_deref(), Some("user-1"));
        assert_eq!(logout.replayable_until, NOW + 3600 + LEEWAY_SECS);
    }

    #[test]
    fn rejects_invalid_claims() {
        let with = |key: &str, value: Value| {
            let mut c = claims();
            c[key] = value;
            c
        };
        let without = |key: &str| {
            let mut c = claims();
            c.as_object_mut().unwrap().remove(key);
            c
        };
        assert_eq!(
            err(&with("iss", json!("https://evil.example"))),
            "wrong issuer"
        );
        assert_eq!(err(&with("aud", json!("someone-else"))), "wrong audience");
        assert_eq!(err(&without("aud")), "wrong audience");
        assert_eq!(err(&without("iat")), "missing iat");
        assert_eq!(err(&with("iat", json!(NOW + 120))), "issued in the future");
        assert_eq!(err(&with("iat", json!(NOW - 3600))), "too old");
        assert_eq!(err(&with("exp", json!(NOW - 120))), "expired");
        assert_eq!(err(&without("events")), "missing logout event");
        assert_eq!(
            err(&with("events", json!({ "urn:other": {} }))),
            "missing logout event"
        );
        assert_eq!(err(&with("nonce", json!("n"))), "nonce is not allowed");
        assert_eq!(err(&without("jti")), "missing jti");
        let mut anonymous = without("sid");
        anonymous.as_object_mut().unwrap().remove("sub");
        assert_eq!(err(&anonymous), "neither sid nor sub");
        assert!(check(&json!({ "alg": "HS256" }), &with("iat", json!(NOW + 30))).is_ok());
    }

    #[test]
    fn rejects_bad_headers_and_signatures() {
        assert_eq!(
            check(&json!({ "alg": "HS256", "typ": "JWT" }), &claims())
                .unwrap_err()
                .0,
            "unexpected typ header"
        );
        let algs = [CoreJwsSigningAlgorithm::RsaSsaPkcs1V15Sha256];
        let token = hs256(&json!({ "alg": "HS256" }), &claims());
        let empty = CoreJsonWebKeySet::new(vec![]);
        assert_eq!(
            validate_with(&token, &empty, &expected(&algs))
                .unwrap_err()
                .0,
            "algorithm not allowed"
        );
        let none = format!(
            "{}.{}.",
            URL_SAFE_NO_PAD.encode(r#"{"alg":"none"}"#),
            URL_SAFE_NO_PAD.encode(claims().to_string())
        );
        let all = [CoreJwsSigningAlgorithm::None];
        assert_eq!(
            validate_with(&none, &empty, &expected(&all)).unwrap_err().0,
            "algorithm not allowed"
        );
        let hs = [CoreJwsSigningAlgorithm::HmacSha256];
        let (signed, _) = token.rsplit_once('.').unwrap();
        let forged = format!("{signed}.{}", URL_SAFE_NO_PAD.encode([0u8; 32]));
        assert_eq!(
            validate_with(&forged, &empty, &expected(&hs))
                .unwrap_err()
                .0,
            "invalid signature"
        );
        assert_eq!(
            validate_with("a.b", &empty, &expected(&hs)).unwrap_err().0,
            "malformed token"
        );
    }
}
