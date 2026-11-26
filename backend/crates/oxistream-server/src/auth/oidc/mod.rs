//! Single sign-on with the one configured `OpenID` Connect provider.

use std::future::Future;
use std::pin::Pin;
use std::sync::{Arc, RwLock};
use std::time::{Duration, Instant};

use base64::Engine;
use base64::engine::general_purpose::URL_SAFE_NO_PAD;
use openidconnect::core::{
    CoreAuthenticationFlow, CoreClient, CoreIdToken, CoreJsonWebKeySet, CoreTokenResponse,
};
use openidconnect::{
    AccessToken, AccessTokenHash, AsyncHttpClient, AuthorizationCode, ClaimsVerificationError,
    ClientId, ClientSecret, CsrfToken, EndpointMaybeSet, EndpointNotSet, EndpointSet, HttpRequest,
    HttpResponse, IssuerUrl, Nonce, OAuth2TokenResponse, PkceCodeChallenge, PkceCodeVerifier,
    ProviderMetadataWithLogout, RedirectUrl, Scope, SignatureVerificationError, TokenResponse,
};
use oxistream_db::Identity;
use reqwest::Url;
use serde_json::Value;
use sha2::{Digest, Sha256};
use tokio::sync::Mutex;

use crate::config::OidcConfig;

pub mod claims;
pub mod pending;

pub use claims::Claims;
use pending::{PendingCodec, PendingLogin};

const CONNECT_TIMEOUT: Duration = Duration::from_secs(5);
const REQUEST_TIMEOUT: Duration = Duration::from_secs(10);
const DISCOVERY_BACKOFF_MIN: Duration = Duration::from_secs(1);
const DISCOVERY_BACKOFF_MAX: Duration = Duration::from_mins(1);
/// Unknown key IDs refetch the JWKS at most this often.
const JWKS_REFRESH_INTERVAL: Duration = Duration::from_secs(30);

type Metadata = ProviderMetadataWithLogout;
type Client = CoreClient<
    EndpointSet,
    EndpointNotSet,
    EndpointNotSet,
    EndpointNotSet,
    EndpointMaybeSet,
    EndpointMaybeSet,
>;

/// Why a sign-in failed; the code is passed to the frontend as `/login?error=`.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum LoginError {
    /// Single sign-on is not configured.
    Disabled,
    /// The provider has not been discovered yet or cannot be reached.
    Unavailable,
    /// The user cancelled or the provider refused.
    Denied,
    /// Expired or forged state, invalid tokens, and other protocol errors.
    Failed,
    AccountDisabled,
}

impl LoginError {
    pub fn code(self) -> &'static str {
        match self {
            Self::Disabled => "oidc_disabled",
            Self::Unavailable => "oidc_unavailable",
            Self::Denied => "oidc_denied",
            Self::Failed => "oidc_failed",
            Self::AccountDisabled => "account_disabled",
        }
    }
}

/// Query parameters of the redirect back from the provider.
#[derive(Debug, Default, Clone, serde::Deserialize, utoipa::IntoParams)]
#[into_params(parameter_in = Query)]
pub struct CallbackParams {
    /// Authorization code.
    pub code: Option<String>,
    pub state: Option<String>,
    /// Error code if the provider did not authorize the request.
    pub error: Option<String>,
    pub error_description: Option<String>,
    /// Issuer identifier (RFC 9207), checked if present.
    pub iss: Option<String>,
}

/// A user authenticated by the provider.
#[derive(Debug, Clone)]
pub struct VerifiedLogin {
    pub identity: Identity,
    /// ID token claims, completed from userinfo if needed.
    pub claims: Claims,
    /// Provider session ID, for back-channel logout.
    pub sid: Option<String>,
    /// Raw ID token, for RP-initiated logout.
    pub id_token: String,
    /// Whether the user is in the admin group; `None` if no admin group is configured.
    pub admin: Option<bool>,
    pub redirect: String,
}

#[derive(Clone)]
pub struct Oidc {
    inner: Arc<Inner>,
}

struct Inner {
    config: OidcConfig,
    http: HttpClient,
    codec: PendingCodec,
    metadata: RwLock<Option<Arc<Metadata>>>,
    jwks_refreshed_at: Mutex<Option<Instant>>,
}

impl Oidc {
    pub fn new(config: OidcConfig) -> Self {
        Self {
            inner: Arc::new(Inner {
                codec: PendingCodec::new(config.state_secret.as_bytes()),
                config,
                http: HttpClient::new(),
                metadata: RwLock::new(None),
                jwks_refreshed_at: Mutex::new(None),
            }),
        }
    }

    pub fn config(&self) -> &OidcConfig {
        &self.inner.config
    }

    /// Whether provider discovery has succeeded.
    pub fn is_ready(&self) -> bool {
        self.metadata().is_some()
    }

    fn metadata(&self) -> Option<Arc<Metadata>> {
        self.inner
            .metadata
            .read()
            .expect("metadata lock poisoned")
            .clone()
    }

    fn issuer(&self) -> &str {
        &self.inner.config.issuer
    }

    /// Fetches the provider metadata and keys.
    pub async fn discover(&self) -> anyhow::Result<()> {
        let issuer = IssuerUrl::new(self.issuer().to_owned())?;
        let metadata = Metadata::discover_async(issuer, &self.inner.http).await?;
        *self.inner.metadata.write().expect("metadata lock poisoned") = Some(Arc::new(metadata));
        Ok(())
    }

    /// Discovers the provider in the background, retrying with backoff until it succeeds.
    /// Until then `/readyz` fails and sign-in answers with `oidc_unavailable`.
    pub fn spawn_discovery(&self) {
        let oidc = self.clone();
        tokio::spawn(async move {
            let mut backoff = DISCOVERY_BACKOFF_MIN;
            loop {
                match oidc.discover().await {
                    Ok(()) => {
                        tracing::info!(issuer = oidc.issuer(), "OIDC provider discovered");
                        return;
                    }
                    Err(err) => tracing::warn!(
                        issuer = oidc.issuer(),
                        error = %format!("{err:#}"),
                        retry_in = ?backoff,
                        "OIDC discovery failed"
                    ),
                }
                tokio::time::sleep(backoff).await;
                backoff = (backoff * 2).min(DISCOVERY_BACKOFF_MAX);
            }
        });
    }

    /// Refetches the provider's keys after an unknown key ID, at most every
    /// [`JWKS_REFRESH_INTERVAL`]. Returns whether new keys were loaded.
    pub async fn refresh_jwks(&self) -> bool {
        let Some(current) = self.metadata() else {
            return false;
        };
        let mut refreshed_at = self.inner.jwks_refreshed_at.lock().await;
        if refreshed_at.is_some_and(|at| at.elapsed() < JWKS_REFRESH_INTERVAL) {
            return false;
        }
        *refreshed_at = Some(Instant::now());
        match CoreJsonWebKeySet::fetch_async(current.jwks_uri(), &self.inner.http).await {
            Ok(jwks) => {
                let mut metadata = (*current).clone();
                metadata = metadata.set_jwks(jwks);
                *self.inner.metadata.write().expect("metadata lock poisoned") =
                    Some(Arc::new(metadata));
                tracing::info!("refreshed OIDC provider keys");
                true
            }
            Err(err) => {
                tracing::warn!(error = %err, "could not refresh OIDC provider keys");
                false
            }
        }
    }

    /// The provider's current signing keys, or `None` before discovery.
    pub fn jwks(&self) -> Option<CoreJsonWebKeySet> {
        self.metadata().map(|metadata| metadata.jwks().clone())
    }

    fn client(&self, metadata: &Metadata) -> Result<Client, LoginError> {
        let config = &self.inner.config;
        let redirect = RedirectUrl::new(config.redirect_url.clone()).map_err(|err| {
            tracing::error!(error = %err, "invalid OIDC redirect URL");
            LoginError::Failed
        })?;
        Ok(Client::from_provider_metadata(
            metadata.clone(),
            ClientId::new(config.client_id.clone()),
            config.client_secret.clone().map(ClientSecret::new),
        )
        .set_redirect_uri(redirect))
    }

    /// The provider URL to send the browser to, and the value of the pending-login cookie.
    pub fn start_login(&self, redirect: String) -> Result<(Url, String), LoginError> {
        let metadata = self.metadata().ok_or(LoginError::Unavailable)?;
        let client = self.client(&metadata)?;
        let (challenge, verifier) = PkceCodeChallenge::new_random_sha256();
        let scopes = self
            .inner
            .config
            .scopes
            .iter()
            .filter(|scope| *scope != "openid")
            .map(|scope| Scope::new(scope.clone()));
        let (url, state, nonce) = client
            .authorize_url(
                CoreAuthenticationFlow::AuthorizationCode,
                CsrfToken::new_random,
                Nonce::new_random,
            )
            .add_scopes(scopes)
            .set_pkce_challenge(challenge)
            .url();
        let value = self.inner.codec.encode(&PendingLogin {
            state: state.into_secret(),
            nonce: nonce.secret().clone(),
            pkce_verifier: verifier.into_secret(),
            redirect,
            exp: pending::now() + pending::TTL.as_secs(),
        });
        Ok((url, value))
    }

    /// The login started by this browser, if its cookie is valid and not expired.
    pub fn pending_login(&self, cookie: Option<&str>) -> Option<PendingLogin> {
        self.inner.codec.decode(cookie?, pending::now())
    }

    /// Checks the callback against the pending login, redeems the code and verifies the
    /// ID token.
    pub async fn finish_login(
        &self,
        params: CallbackParams,
        pending: PendingLogin,
    ) -> Result<VerifiedLogin, LoginError> {
        let failed = |reason: &str| {
            tracing::warn!(reason, "OIDC sign-in rejected");
            LoginError::Failed
        };
        if !digest_eq(params.state.as_deref().unwrap_or_default(), &pending.state) {
            return Err(failed("state mismatch"));
        }
        if let Some(error) = params.error {
            tracing::info!(
                %error,
                description = params.error_description.as_deref().unwrap_or_default(),
                "OIDC provider returned an error"
            );
            return Err(if error == "access_denied" {
                LoginError::Denied
            } else {
                LoginError::Failed
            });
        }
        if params
            .iss
            .as_deref()
            .is_some_and(|iss| iss != self.issuer())
        {
            return Err(failed("issuer mismatch in callback"));
        }
        let code = params.code.ok_or_else(|| failed("missing code"))?;

        let metadata = self.metadata().ok_or(LoginError::Unavailable)?;
        let client = self.client(&metadata)?;
        let token: CoreTokenResponse = client
            .exchange_code(AuthorizationCode::new(code))
            .map_err(|_| failed("provider has no token endpoint"))?
            .set_pkce_verifier(PkceCodeVerifier::new(pending.pkce_verifier))
            .request_async(&self.inner.http)
            .await
            .map_err(|err| {
                tracing::warn!(error = %format!("{err:?}"), "OIDC token request failed");
                LoginError::Failed
            })?;
        let id_token = token
            .id_token()
            .ok_or_else(|| failed("no ID token in token response"))?
            .clone();
        let nonce = Nonce::new(pending.nonce);

        let client = match Self::verify_id_token(&client, &id_token, token.access_token(), &nonce) {
            Err(ClaimsVerificationError::SignatureVerification(
                SignatureVerificationError::NoMatchingKey,
            )) if self.refresh_jwks().await => {
                let metadata = self.metadata().ok_or(LoginError::Unavailable)?;
                self.client(&metadata)?
            }
            Err(err) => {
                tracing::warn!(error = %err, "invalid OIDC ID token");
                return Err(LoginError::Failed);
            }
            Ok(()) => client,
        };
        Self::verify_id_token(&client, &id_token, token.access_token(), &nonce).map_err(|err| {
            tracing::warn!(error = %err, "invalid OIDC ID token");
            LoginError::Failed
        })?;

        let raw = id_token.to_string();
        let mut claims = jwt_payload(&raw).ok_or_else(|| failed("unreadable ID token payload"))?;
        let subject = claims
            .get("sub")
            .and_then(Value::as_str)
            .ok_or_else(|| failed("no subject"))?
            .to_owned();
        if self.needs_userinfo(&claims) {
            self.complete_from_userinfo(&metadata, token.access_token(), &subject, &mut claims)
                .await;
        }
        let config = &self.inner.config;
        let admin = config.admin_group.as_ref().map(|group| {
            claims::groups(&claims, &config.groups_claim)
                .unwrap_or_default()
                .contains(group)
        });
        Ok(VerifiedLogin {
            identity: Identity {
                issuer: self.issuer().to_owned(),
                subject,
            },
            sid: claims.get("sid").and_then(Value::as_str).map(str::to_owned),
            claims,
            id_token: raw,
            admin,
            redirect: pending.redirect,
        })
    }

    fn verify_id_token(
        client: &Client,
        id_token: &CoreIdToken,
        access_token: &AccessToken,
        nonce: &Nonce,
    ) -> Result<(), ClaimsVerificationError> {
        let verifier = client.id_token_verifier();
        let claims = id_token.claims(&verifier, nonce)?;
        if let Some(expected) = claims.access_token_hash() {
            let alg = id_token
                .signing_alg()
                .map_err(ClaimsVerificationError::SignatureVerification)?;
            let key = id_token
                .signing_key(&verifier)
                .map_err(ClaimsVerificationError::SignatureVerification)?;
            let actual = AccessTokenHash::from_token(access_token, alg, key)
                .map_err(|err| ClaimsVerificationError::Other(err.to_string()))?;
            if actual != *expected {
                return Err(ClaimsVerificationError::Other(
                    "access token hash mismatch".into(),
                ));
            }
        }
        Ok(())
    }

    /// Userinfo is only fetched when the ID token lacks the groups or any username claim.
    fn needs_userinfo(&self, claims: &Claims) -> bool {
        let config = &self.inner.config;
        let groups_missing =
            config.admin_group.is_some() && claims::groups(claims, &config.groups_claim).is_none();
        let username_missing = ["preferred_username", "name", "email"]
            .iter()
            .all(|claim| !claims.contains_key(*claim));
        groups_missing || username_missing
    }

    async fn complete_from_userinfo(
        &self,
        metadata: &Metadata,
        access_token: &AccessToken,
        subject: &str,
        claims: &mut Claims,
    ) {
        let Some(endpoint) = metadata.userinfo_endpoint() else {
            return;
        };
        let response = self
            .inner
            .http
            .client
            .get(endpoint.url().clone())
            .bearer_auth(access_token.secret())
            .header(reqwest::header::ACCEPT, "application/json")
            .send()
            .await
            .and_then(reqwest::Response::error_for_status);
        let info: Claims = match response {
            Ok(response) => match response
                .bytes()
                .await
                .map(|body| serde_json::from_slice(&body))
            {
                Ok(Ok(info)) => info,
                Ok(Err(err)) => {
                    tracing::warn!(error = %err, "unreadable OIDC userinfo response");
                    return;
                }
                Err(err) => {
                    tracing::warn!(error = %err, "OIDC userinfo request failed");
                    return;
                }
            },
            Err(err) => {
                tracing::warn!(error = %err, "OIDC userinfo request failed");
                return;
            }
        };
        if info.get("sub").and_then(Value::as_str) != Some(subject) {
            tracing::warn!("OIDC userinfo subject does not match the ID token");
            return;
        }
        for (key, value) in info {
            claims.entry(key).or_insert(value);
        }
    }

    /// The provider's end-session URL for RP-initiated logout, if it has one.
    pub fn end_session_url(&self, id_token: &str) -> Option<String> {
        let metadata = self.metadata()?;
        let endpoint = metadata
            .additional_metadata()
            .end_session_endpoint
            .as_ref()?;
        let config = &self.inner.config;
        let mut url = endpoint.url().clone();
        url.query_pairs_mut()
            .append_pair("id_token_hint", id_token)
            .append_pair("post_logout_redirect_uri", &config.post_logout_redirect_url)
            .append_pair("client_id", &config.client_id);
        Some(url.into())
    }
}

fn digest_eq(a: &str, b: &str) -> bool {
    Sha256::digest(a) == Sha256::digest(b)
}

/// The claims of a compact JWT, without verifying it.
pub fn jwt_payload(jwt: &str) -> Option<Claims> {
    let payload = jwt.split('.').nth(1)?;
    serde_json::from_slice(&URL_SAFE_NO_PAD.decode(payload).ok()?).ok()
}

/// A same-origin path to continue to after sign-in; anything else becomes `/`.
pub fn safe_redirect(target: Option<&str>) -> String {
    let Some(target) = target.filter(|t| {
        t.starts_with('/') && !t.starts_with("//") && !t.starts_with("/\\") && !t.contains('\\')
    }) else {
        return "/".to_owned();
    };
    let base = Url::parse("http://oxistream.invalid/").expect("valid base URL");
    match base.join(target) {
        Ok(url)
            if url.origin() == base.origin()
                && url.path() != "/login"
                && url.path() != "/register" =>
        {
            let mut path = url.path().to_owned();
            if let Some(query) = url.query() {
                path.push('?');
                path.push_str(query);
            }
            path
        }
        _ => "/".to_owned(),
    }
}

/// [`AsyncHttpClient`] on our reqwest; redirects are not followed, as the OAuth 2.0
/// security advice requires.
#[derive(Clone)]
struct HttpClient {
    client: reqwest::Client,
}

impl HttpClient {
    fn new() -> Self {
        let client = reqwest::Client::builder()
            .tls_backend_preconfigured(
                crate::proxy::tls_config().expect("ring supports the default TLS versions"),
            )
            .connect_timeout(CONNECT_TIMEOUT)
            .timeout(REQUEST_TIMEOUT)
            .redirect(reqwest::redirect::Policy::none())
            .build()
            .expect("build OIDC HTTP client");
        Self { client }
    }
}

impl<'c> AsyncHttpClient<'c> for HttpClient {
    type Error = reqwest::Error;
    type Future = Pin<Box<dyn Future<Output = Result<HttpResponse, Self::Error>> + Send + 'c>>;

    fn call(&'c self, request: HttpRequest) -> Self::Future {
        Box::pin(async move {
            let (parts, body) = request.into_parts();
            let response = self
                .client
                .request(parts.method, parts.uri.to_string())
                .headers(parts.headers)
                .body(body)
                .send()
                .await?;
            let mut builder = axum::http::Response::builder().status(response.status());
            if let Some(headers) = builder.headers_mut() {
                *headers = response.headers().clone();
            }
            let body = response.bytes().await?.to_vec();
            Ok(builder.body(body).expect("valid response parts"))
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn redirect_targets_stay_on_this_origin() {
        for (target, expected) in [
            (Some("/feed/history"), "/feed/history"),
            (Some("/watch?v=dQw4w9WgXcQ"), "/watch?v=dQw4w9WgXcQ"),
            (Some("/a/../b"), "/b"),
            (None, "/"),
            (Some(""), "/"),
            (Some("https://evil.example/"), "/"),
            (Some("//evil.example"), "/"),
            (Some("/\\evil.example"), "/"),
            (Some("/x\\y"), "/"),
            (Some("/login?redirect=/"), "/"),
            (Some("/register"), "/"),
        ] {
            assert_eq!(safe_redirect(target), expected, "{target:?}");
        }
    }

    #[test]
    fn reads_unverified_jwt_payloads() {
        let payload = URL_SAFE_NO_PAD.encode(br#"{"sub":"abc","sid":"s1"}"#);
        let claims = jwt_payload(&format!("e30.{payload}.sig")).unwrap();
        assert_eq!(claims["sub"], "abc");
        assert_eq!(jwt_payload("nope"), None);
    }

    #[test]
    fn login_error_codes() {
        assert_eq!(LoginError::Failed.code(), "oidc_failed");
        assert_eq!(LoginError::AccountDisabled.code(), "account_disabled");
    }
}
