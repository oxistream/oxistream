//! Local accounts, single sign-on and server-side sessions.

use std::net::IpAddr;
use std::sync::Arc;
use std::time::Duration;

use axum::extract::{FromRequestParts, Request, State};
use axum::http::request::Parts;
use axum::http::{HeaderMap, HeaderValue, header};
use axum::middleware::Next;
use axum::response::{IntoResponse, Response};
use oxistream_db::{ActiveSession, NewSession, SessionRepository, TokenHash, User, UserRepository};
use time::OffsetDateTime;

use crate::AppState;
use crate::api::error::ApiError;
use crate::config::{AuthConfig, RateLimitConfig};
use crate::rate_limit::{ClientIp, LoginLimiter};

pub mod cookie;
pub mod csrf;
pub mod oidc;
pub mod password;

pub use password::PasswordHasher;

/// Sessions are renewed at most this often, so active users do not cause a write per request.
pub const RENEW_INTERVAL: Duration = Duration::from_mins(5);
const CLEANUP_INTERVAL: Duration = Duration::from_hours(1);
const MAX_USER_AGENT_LEN: usize = 512;
/// Called server to server by the provider, so it carries no `Origin` and no session; the
/// signed logout token is its authentication.
pub const BACKCHANNEL_LOGOUT_PATH: &str = "/api/v0/auth/oidc/backchannel-logout";

#[derive(Clone)]
pub struct Auth {
    inner: Arc<Inner>,
}

struct Inner {
    config: AuthConfig,
    users: Arc<dyn UserRepository>,
    sessions: Arc<dyn SessionRepository>,
    hasher: PasswordHasher,
    client_ip: ClientIp,
    /// `None` when rate limiting is disabled.
    login_limiter: Option<LoginLimiter>,
    oidc: Option<oidc::Oidc>,
}

impl Auth {
    pub fn new(
        config: AuthConfig,
        users: Arc<dyn UserRepository>,
        sessions: Arc<dyn SessionRepository>,
        rate_limit: Option<&RateLimitConfig>,
    ) -> Self {
        let oidc = config.oidc.clone().map(oidc::Oidc::new);
        Self {
            inner: Arc::new(Inner {
                oidc,
                config,
                users,
                sessions,
                hasher: PasswordHasher::default(),
                client_ip: ClientIp::new(rate_limit.and_then(|rl| rl.trusted_header.clone())),
                login_limiter: rate_limit.map(|_| LoginLimiter::default()),
            }),
        }
    }

    pub fn config(&self) -> &AuthConfig {
        &self.inner.config
    }

    pub fn users(&self) -> &dyn UserRepository {
        self.inner.users.as_ref()
    }

    pub fn sessions(&self) -> &dyn SessionRepository {
        self.inner.sessions.as_ref()
    }

    /// `None` when single sign-on is not configured.
    pub fn oidc(&self) -> Option<&oidc::Oidc> {
        self.inner.oidc.as_ref()
    }

    pub fn hasher(&self) -> &PasswordHasher {
        &self.inner.hasher
    }

    /// Records a login or registration attempt; `false` if over the limit.
    pub fn allow_attempt(&self, ip: Option<IpAddr>, username: Option<&str>) -> bool {
        self.inner
            .login_limiter
            .as_ref()
            .is_none_or(|limiter| limiter.check(ip, username))
    }

    /// Starts a session for `user` and returns the cookie that carries it.
    pub async fn start_session(
        &self,
        user: &User,
        headers: &HeaderMap,
        ip: Option<IpAddr>,
    ) -> Result<HeaderValue, ApiError> {
        self.start_oidc_session(user, headers, ip, None, None).await
    }

    /// Like [`Self::start_session`], remembering the provider's session ID and ID token.
    pub async fn start_oidc_session(
        &self,
        user: &User,
        headers: &HeaderMap,
        ip: Option<IpAddr>,
        oidc_sid: Option<String>,
        oidc_id_token: Option<String>,
    ) -> Result<HeaderValue, ApiError> {
        if let Some(old) = cookie::session_token(headers).and_then(cookie::hash_token) {
            self.sessions().delete(&old).await?;
        }
        let (token, token_hash) = cookie::new_token();
        let user_agent = headers
            .get(header::USER_AGENT)
            .and_then(|value| value.to_str().ok())
            .map(|ua| ua.chars().take(MAX_USER_AGENT_LEN).collect());
        self.sessions()
            .create(&NewSession {
                token_hash,
                user_id: user.id,
                expires_at: self.expiry_from_now(),
                user_agent,
                ip,
                oidc_sid,
                oidc_id_token,
            })
            .await?;
        Ok(self.session_cookie(&token))
    }

    pub fn session_cookie(&self, token: &str) -> HeaderValue {
        cookie::session_cookie(
            token,
            self.config().session_ttl,
            self.config().cookie_secure,
        )
    }

    pub fn clear_cookie(&self) -> HeaderValue {
        cookie::clear_session_cookie(self.config().cookie_secure)
    }

    fn expiry_from_now(&self) -> OffsetDateTime {
        OffsetDateTime::now_utc() + self.config().session_ttl
    }

    /// Resolves a valid session and renews it if it was last renewed a while ago.
    async fn resolve(&self, token_hash: &TokenHash) -> Result<Option<(User, bool)>, ApiError> {
        let Some(ActiveSession { session, user }) = self.sessions().find_active(token_hash).await?
        else {
            return Ok(None);
        };
        let renew = OffsetDateTime::now_utc() - session.last_seen_at >= RENEW_INTERVAL;
        if renew
            && let Err(err) = self
                .sessions()
                .touch(token_hash, self.expiry_from_now())
                .await
        {
            tracing::warn!(error = ?err, "could not renew session");
            return Ok(Some((user, false)));
        }
        Ok(Some((user, renew)))
    }

    /// Periodically deletes expired sessions and forgets idle login rate limit entries.
    pub fn spawn_cleanup(&self) {
        let auth = self.clone();
        tokio::spawn(async move {
            let mut interval = tokio::time::interval(CLEANUP_INTERVAL);
            loop {
                interval.tick().await;
                match auth.sessions().delete_expired().await {
                    Ok(0) => {}
                    Ok(count) => tracing::debug!(count, "deleted expired sessions"),
                    Err(err) => tracing::warn!(error = ?err, "could not delete expired sessions"),
                }
                if auth.oidc().is_some()
                    && let Err(err) = auth.sessions().delete_expired_logout_jtis().await
                {
                    tracing::warn!(error = ?err, "could not delete expired logout token IDs");
                }
                if let Some(limiter) = &auth.inner.login_limiter {
                    limiter.retain_recent();
                }
            }
        });
    }
}

/// The signed-in user; rejects anonymous requests with 401.
#[derive(Debug, Clone)]
pub struct CurrentUser(pub User);

/// The signed-in user, if any.
#[derive(Debug, Clone)]
pub struct OptionalUser(pub Option<User>);

impl<S: Send + Sync> FromRequestParts<S> for CurrentUser {
    type Rejection = ApiError;

    fn from_request_parts(
        parts: &mut Parts,
        _state: &S,
    ) -> impl Future<Output = Result<Self, Self::Rejection>> + Send {
        std::future::ready(
            parts
                .extensions
                .get::<Self>()
                .cloned()
                .ok_or_else(ApiError::unauthorized),
        )
    }
}

impl<S: Send + Sync> FromRequestParts<S> for OptionalUser {
    type Rejection = std::convert::Infallible;

    fn from_request_parts(
        parts: &mut Parts,
        _state: &S,
    ) -> impl Future<Output = Result<Self, Self::Rejection>> + Send {
        let user = parts.extensions.get::<CurrentUser>().cloned().map(|u| u.0);
        std::future::ready(Ok(Self(user)))
    }
}

/// The client IP, from the trusted proxy header if configured.
#[derive(Debug, Clone, Copy)]
pub struct ClientAddr(pub Option<IpAddr>);

impl FromRequestParts<AppState> for ClientAddr {
    type Rejection = std::convert::Infallible;

    fn from_request_parts(
        parts: &mut Parts,
        state: &AppState,
    ) -> impl Future<Output = Result<Self, Self::Rejection>> + Send {
        let ip = state
            .auth
            .inner
            .client_ip
            .of(&parts.headers, &parts.extensions);
        std::future::ready(Ok(Self(ip)))
    }
}

/// Paths that need a session when `OXISTREAM_REQUIRE_LOGIN` is set.
fn requires_login(path: &str) -> bool {
    let protected = path.starts_with("/api/v0/") || path.starts_with("/proxy/");
    let exempt = path.starts_with("/api/v0/auth/") || path == "/api/v0/instance";
    protected && !exempt
}

/// Checks the origin of state-changing requests, resolves the session cookie into a
/// [`CurrentUser`], renews sessions and enforces `OXISTREAM_REQUIRE_LOGIN`.
pub async fn middleware(State(state): State<AppState>, mut req: Request, next: Next) -> Response {
    let auth = &state.auth;
    let config = auth.config();
    if !csrf::is_safe_method(req.method())
        && req.uri().path() != BACKCHANNEL_LOGOUT_PATH
        && !csrf::origin_allowed(req.headers(), req.uri(), config.public_origin.as_deref())
    {
        return ApiError::origin_rejected().into_response();
    }

    let path = req.uri().path();
    let login_required = config.require_login && requires_login(path);
    // Proxy requests are frequent and need no user unless login is required.
    let wants_user = login_required || path.starts_with("/api/");
    let token = wants_user
        .then(|| cookie::session_token(req.headers()))
        .flatten()
        .map(|token| (token.to_owned(), cookie::hash_token(token)));

    let mut set_cookie = None;
    if let Some((token, hash)) = token {
        let resolved = match hash {
            Some(hash) => match auth.resolve(&hash).await {
                Ok(resolved) => resolved,
                Err(err) => return err.into_response(),
            },
            None => None,
        };
        match resolved {
            Some((user, renewed)) => {
                if renewed {
                    set_cookie = Some(auth.session_cookie(&token));
                }
                req.extensions_mut().insert(CurrentUser(user));
            }
            None => set_cookie = Some(auth.clear_cookie()),
        }
    }

    let mut response = if login_required && req.extensions().get::<CurrentUser>().is_none() {
        ApiError::unauthorized().into_response()
    } else {
        next.run(req).await
    };
    if let Some(value) = set_cookie
        && !response.headers().contains_key(header::SET_COOKIE)
    {
        response.headers_mut().insert(header::SET_COOKIE, value);
    }
    response
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn login_requirement_spares_auth_and_instance() {
        for path in [
            "/api/v0/search",
            "/api/v0/videos/x",
            "/proxy/image/x",
            "/proxy/stream/x/1",
        ] {
            assert!(requires_login(path), "{path}");
        }
        for path in [
            "/api/v0/auth/login",
            "/api/v0/auth/me",
            "/api/v0/instance",
            "/api/openapi.json",
            "/api/docs",
            "/healthz",
            "/watch",
        ] {
            assert!(!requires_login(path), "{path}");
        }
    }
}
