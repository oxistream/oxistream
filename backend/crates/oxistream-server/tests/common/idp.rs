//! An in-process `OpenID` Connect provider for tests: discovery, JWKS, token and userinfo
//! endpoints, signing with RSA keys generated per test run.

use std::collections::HashMap;
use std::sync::{Arc, Mutex, OnceLock};
use std::time::{SystemTime, UNIX_EPOCH};

use axum::extract::{Form, State};
use axum::http::{HeaderMap, StatusCode, header};
use axum::response::{IntoResponse, Response};
use axum::routing::{get, post};
use axum::{Json, Router};
use base64::Engine;
use base64::engine::general_purpose::URL_SAFE_NO_PAD;
use openidconnect::core::{CoreJwsSigningAlgorithm, CoreRsaPrivateSigningKey};
use openidconnect::{JsonWebKeyId, PrivateSigningKey};
use rsa::pkcs1::EncodeRsaPrivateKey;
use serde_json::{Value, json};
use sha2::{Digest, Sha256};

pub const CLIENT_ID: &str = "oxistream";

/// RSA keys are slow to generate, so each test binary makes two and shares them.
fn pems() -> &'static [String; 2] {
    static PEMS: OnceLock<[String; 2]> = OnceLock::new();
    PEMS.get_or_init(|| {
        std::array::from_fn(|_| {
            use rsa::Generate;
            rsa::PrivateKey::generate()
                .to_pkcs1_pem(rsa::pkcs1::LineEnding::LF)
                .unwrap()
                .to_string()
        })
    })
}

pub struct SigningKey {
    pub kid: String,
    key: CoreRsaPrivateSigningKey,
}

impl SigningKey {
    /// Key `index` (0 or 1) with the given key ID.
    pub fn new(index: usize, kid: &str) -> Self {
        Self {
            kid: kid.to_owned(),
            key: CoreRsaPrivateSigningKey::from_pem(
                &pems()[index],
                Some(JsonWebKeyId::new(kid.to_owned())),
            )
            .unwrap(),
        }
    }

    pub fn jwk(&self) -> Value {
        serde_json::to_value(self.key.as_verification_key()).unwrap()
    }

    /// A compact RS256 JWS of `claims` with the given extra header fields.
    pub fn sign(&self, header: &Value, claims: &Value) -> String {
        let mut full = json!({ "alg": "RS256", "kid": self.kid });
        full.as_object_mut()
            .unwrap()
            .extend(header.as_object().cloned().unwrap_or_default());
        let input = format!(
            "{}.{}",
            URL_SAFE_NO_PAD.encode(full.to_string()),
            URL_SAFE_NO_PAD.encode(claims.to_string())
        );
        let signature = self
            .key
            .sign(
                &CoreJwsSigningAlgorithm::RsaSsaPkcs1V15Sha256,
                input.as_bytes(),
            )
            .unwrap();
        format!("{input}.{}", URL_SAFE_NO_PAD.encode(signature))
    }
}

pub fn now() -> u64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap()
        .as_secs()
}

/// What the provider issues for an authorization code.
#[derive(Clone)]
pub struct Grant {
    /// The PKCE challenge from the authorization request.
    pub code_challenge: String,
    /// Claims of the ID token; `iss`, `aud`, `iat` and `exp` are added unless present.
    pub claims: Value,
}

#[derive(Default)]
pub struct IdpState {
    pub issuer: String,
    pub keys: Vec<SigningKey>,
    /// Index into `keys` used for signing.
    pub active: usize,
    pub grants: HashMap<String, Grant>,
    /// Userinfo by subject.
    pub userinfo: HashMap<String, Value>,
    pub end_session: bool,
    pub jwks_fetches: usize,
}

#[derive(Clone)]
pub struct Idp {
    pub issuer: String,
    pub state: Arc<Mutex<IdpState>>,
}

impl Idp {
    pub async fn start() -> Self {
        let state = Arc::new(Mutex::new(IdpState {
            keys: vec![SigningKey::new(0, "key-1")],
            end_session: true,
            ..IdpState::default()
        }));
        let router = Router::new()
            .route("/.well-known/openid-configuration", get(discovery))
            .route("/jwks", get(jwks))
            .route("/token", post(token))
            .route("/userinfo", get(userinfo))
            .with_state(state.clone());
        let base = super::spawn_upstream(router).await;
        let issuer = base.as_str().trim_end_matches('/').to_owned();
        state.lock().unwrap().issuer.clone_from(&issuer);
        Self { issuer, state }
    }

    /// Registers an authorization code for `claims`.
    pub fn grant(&self, code_challenge: &str, claims: Value) -> String {
        let code = format!("code-{}", super::unique_suffix());
        self.state.lock().unwrap().grants.insert(
            code.clone(),
            Grant {
                code_challenge: code_challenge.to_owned(),
                claims,
            },
        );
        code
    }

    /// Completes claims with the registered defaults and signs them with the active key.
    pub fn id_token(&self, claims: &Value) -> String {
        let state = self.state.lock().unwrap();
        let mut full = json!({
            "iss": state.issuer,
            "aud": CLIENT_ID,
            "iat": now(),
            "exp": now() + 300,
        });
        full.as_object_mut()
            .unwrap()
            .extend(claims.as_object().cloned().unwrap_or_default());
        state.keys[state.active].sign(&json!({ "typ": "JWT" }), &full)
    }
}

type Shared = State<Arc<Mutex<IdpState>>>;

async fn discovery(State(state): Shared) -> Json<Value> {
    let state = state.lock().unwrap();
    let issuer = &state.issuer;
    let mut metadata = json!({
        "issuer": issuer,
        "authorization_endpoint": format!("{issuer}/authorize"),
        "token_endpoint": format!("{issuer}/token"),
        "userinfo_endpoint": format!("{issuer}/userinfo"),
        "jwks_uri": format!("{issuer}/jwks"),
        "response_types_supported": ["code"],
        "subject_types_supported": ["public"],
        "id_token_signing_alg_values_supported": ["RS256"],
        "backchannel_logout_supported": true,
        "backchannel_logout_session_supported": true,
    });
    if state.end_session {
        metadata["end_session_endpoint"] = json!(format!("{issuer}/logout"));
    }
    Json(metadata)
}

async fn jwks(State(state): Shared) -> Json<Value> {
    let mut state = state.lock().unwrap();
    state.jwks_fetches += 1;
    Json(json!({ "keys": state.keys.iter().map(SigningKey::jwk).collect::<Vec<_>>() }))
}

fn oauth_error(error: &str) -> Response {
    (StatusCode::BAD_REQUEST, Json(json!({ "error": error }))).into_response()
}

async fn token(
    State(state): Shared,
    headers: HeaderMap,
    Form(form): Form<HashMap<String, String>>,
) -> Response {
    let basic_client = headers
        .get(header::AUTHORIZATION)
        .and_then(|v| v.to_str().ok())
        .and_then(|v| v.strip_prefix("Basic "))
        .and_then(|v| base64::engine::general_purpose::STANDARD.decode(v).ok())
        .and_then(|v| String::from_utf8(v).ok())
        .and_then(|v| v.split_once(':').map(|(id, _)| id.to_owned()));
    let client_id = basic_client.or_else(|| form.get("client_id").cloned());
    if client_id.as_deref() != Some(CLIENT_ID) {
        return oauth_error("invalid_client");
    }
    if form.get("grant_type").map(String::as_str) != Some("authorization_code") {
        return oauth_error("unsupported_grant_type");
    }
    let grant = {
        let mut state = state.lock().unwrap();
        form.get("code").and_then(|code| state.grants.remove(code))
    };
    let Some(grant) = grant else {
        return oauth_error("invalid_grant");
    };
    let verifier = form.get("code_verifier").cloned().unwrap_or_default();
    if URL_SAFE_NO_PAD.encode(Sha256::digest(verifier.as_bytes())) != grant.code_challenge {
        return oauth_error("invalid_grant");
    }
    let issuer = state.lock().unwrap().issuer.clone();
    let idp = Idp {
        issuer,
        state: state.clone(),
    };
    let id_token = idp.id_token(&grant.claims);
    let sub = grant.claims["sub"].as_str().unwrap_or_default();
    Json(json!({
        "access_token": format!("at-{sub}"),
        "token_type": "Bearer",
        "expires_in": 300,
        "id_token": id_token,
    }))
    .into_response()
}

async fn userinfo(State(state): Shared, headers: HeaderMap) -> Response {
    let sub = headers
        .get(header::AUTHORIZATION)
        .and_then(|v| v.to_str().ok())
        .and_then(|v| v.strip_prefix("Bearer at-"))
        .map(str::to_owned);
    let state = state.lock().unwrap();
    match sub.and_then(|sub| state.userinfo.get(&sub)) {
        Some(info) => Json(info.clone()).into_response(),
        None => StatusCode::UNAUTHORIZED.into_response(),
    }
}
