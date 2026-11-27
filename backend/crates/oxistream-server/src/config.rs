use std::fmt;
use std::net::SocketAddr;
use std::num::NonZeroU32;
use std::path::PathBuf;
use std::str::FromStr;
use std::time::Duration;

use axum::http::HeaderName;
use rustypipe::param::Country;

pub use crate::retention::Retention;

const BIND: &str = "OXISTREAM_BIND";
const DATABASE_URL: &str = "OXISTREAM_DATABASE_URL";
const DATABASE_MAX_CONNECTIONS: &str = "OXISTREAM_DATABASE_MAX_CONNECTIONS";
const LOG: &str = "OXISTREAM_LOG";
const LOG_FORMAT: &str = "OXISTREAM_LOG_FORMAT";
const BOTGUARD_BIN: &str = "OXISTREAM_BOTGUARD_BIN";
const YOUTUBE_CACHE_DIR: &str = "OXISTREAM_YOUTUBE_CACHE_DIR";
const TRENDING_REGION: &str = "OXISTREAM_TRENDING_REGION";
const PROXY_SECRET: &str = "OXISTREAM_PROXY_SECRET";
const PROXY_STREAM_TTL: &str = "OXISTREAM_PROXY_STREAM_TTL_SECS";
const PROXY_IMAGE_TTL: &str = "OXISTREAM_PROXY_IMAGE_TTL_SECS";
const RATE_LIMIT: &str = "OXISTREAM_RATE_LIMIT";
const RATE_LIMIT_PER_SECOND: &str = "OXISTREAM_RATE_LIMIT_PER_SECOND";
const RATE_LIMIT_BURST: &str = "OXISTREAM_RATE_LIMIT_BURST";
const TRUSTED_PROXY_HEADER: &str = "OXISTREAM_TRUSTED_PROXY_HEADER";
const PUBLIC_URL: &str = "OXISTREAM_PUBLIC_URL";
const AUTH_LOCAL_ENABLED: &str = "OXISTREAM_AUTH_LOCAL_ENABLED";
const REGISTRATION_ENABLED: &str = "OXISTREAM_REGISTRATION_ENABLED";
const REQUIRE_LOGIN: &str = "OXISTREAM_REQUIRE_LOGIN";
const SESSION_TTL_DAYS: &str = "OXISTREAM_SESSION_TTL_DAYS";
const COOKIE_SECURE: &str = "OXISTREAM_COOKIE_SECURE";
const OIDC_ISSUER: &str = "OXISTREAM_OIDC_ISSUER";
const OIDC_CLIENT_ID: &str = "OXISTREAM_OIDC_CLIENT_ID";
const OIDC_CLIENT_SECRET: &str = "OXISTREAM_OIDC_CLIENT_SECRET";
const OIDC_SCOPES: &str = "OXISTREAM_OIDC_SCOPES";
const OIDC_NAME: &str = "OXISTREAM_OIDC_NAME";
const OIDC_ADMIN_GROUP: &str = "OXISTREAM_OIDC_ADMIN_GROUP";
const OIDC_GROUPS_CLAIM: &str = "OXISTREAM_OIDC_GROUPS_CLAIM";
const FEED_INTERVAL: &str = "OXISTREAM_FEED_INTERVAL_MINUTES";
const SPONSORBLOCK_ENABLED: &str = "OXISTREAM_SPONSORBLOCK_ENABLED";
const SPONSORBLOCK_API_URL: &str = "OXISTREAM_SPONSORBLOCK_API_URL";
const HISTORY_MAX_RETENTION: &str = "OXISTREAM_HISTORY_MAX_RETENTION";

pub const MIN_PROXY_SECRET_LEN: usize = 32;

#[derive(Debug, thiserror::Error, PartialEq, Eq)]
pub enum ConfigError {
    #[error("{0} is required but not set")]
    Missing(&'static str),
    #[error("{var} is required when {because} is set")]
    MissingFor {
        var: &'static str,
        because: &'static str,
    },
    #[error("{var} has invalid value {value:?}: {reason}")]
    Invalid {
        var: &'static str,
        value: String,
        reason: String,
    },
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum LogFormat {
    #[default]
    Pretty,
    Json,
}

impl FromStr for LogFormat {
    type Err = String;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s.to_ascii_lowercase().as_str() {
            "pretty" => Ok(Self::Pretty),
            "json" => Ok(Self::Json),
            _ => Err("expected `pretty` or `json`".into()),
        }
    }
}

#[derive(Clone)]
pub struct Config {
    pub bind: SocketAddr,
    pub database_url: String,
    pub database_max_connections: u32,
    pub log_filter: String,
    pub log_format: LogFormat,
    pub youtube: YouTubeConfig,
    pub proxy: ProxyConfig,
    /// `None` disables rate limiting.
    pub rate_limit: Option<RateLimitConfig>,
    pub auth: AuthConfig,
    pub feed: FeedConfig,
    pub sponsorblock: SponsorBlockConfig,
    /// Longest watch history retention users may pick.
    pub history_max_retention: Retention,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SponsorBlockConfig {
    pub enabled: bool,
    /// Base URL of the SponsorBlock API server, without a trailing slash.
    pub api_url: String,
}

impl SponsorBlockConfig {
    pub const DEFAULT_API_URL: &str = "https://sponsor.ajay.app";
}

impl Default for SponsorBlockConfig {
    fn default() -> Self {
        Self {
            enabled: true,
            api_url: Self::DEFAULT_API_URL.to_owned(),
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct FeedConfig {
    /// How often each subscribed channel is fetched.
    pub interval: Duration,
}

impl FeedConfig {
    pub const DEFAULT_INTERVAL: Duration = Duration::from_mins(30);
}

impl Default for FeedConfig {
    fn default() -> Self {
        Self {
            interval: Self::DEFAULT_INTERVAL,
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
#[allow(clippy::struct_excessive_bools)]
pub struct AuthConfig {
    pub local_enabled: bool,
    pub registration_enabled: bool,
    /// Rejects anonymous API and proxy requests.
    pub require_login: bool,
    /// Sliding lifetime of a session since its last use.
    pub session_ttl: Duration,
    pub cookie_secure: bool,
    /// Origin (`scheme://host[:port]`) browsers use to reach this instance. When unset,
    /// state-changing requests must come from the origin named by the `Host` header.
    pub public_origin: Option<String>,
    /// `None` disables single sign-on.
    pub oidc: Option<OidcConfig>,
}

/// The single `OpenID` Connect provider.
#[derive(Clone, PartialEq, Eq)]
pub struct OidcConfig {
    pub issuer: String,
    pub client_id: String,
    /// `None` for public clients.
    pub client_secret: Option<String>,
    pub scopes: Vec<String>,
    /// Label of the sign-in button.
    pub name: String,
    /// Members of this group are admins; when unset, roles are not synced.
    pub admin_group: Option<String>,
    pub groups_claim: String,
    /// `{public origin}/api/v0/auth/oidc/callback`.
    pub redirect_url: String,
    /// Where the provider sends the browser after RP-initiated logout.
    pub post_logout_redirect_url: String,
    /// HMAC key for the pending-login cookie; shared by all replicas.
    pub state_secret: String,
}

impl OidcConfig {
    pub const DEFAULT_SCOPES: &str = "openid profile email";
    pub const DEFAULT_NAME: &str = "SSO";
    pub const DEFAULT_GROUPS_CLAIM: &str = "groups";
    pub const CALLBACK_PATH: &str = "/api/v0/auth/oidc/callback";

    /// A configuration with defaults for everything that is not given.
    pub fn new(
        issuer: impl Into<String>,
        client_id: impl Into<String>,
        public_origin: &str,
        state_secret: impl Into<String>,
    ) -> Self {
        Self {
            issuer: issuer.into(),
            client_id: client_id.into(),
            client_secret: None,
            scopes: parse_scopes(Self::DEFAULT_SCOPES),
            name: Self::DEFAULT_NAME.to_owned(),
            admin_group: None,
            groups_claim: Self::DEFAULT_GROUPS_CLAIM.to_owned(),
            redirect_url: format!("{public_origin}{}", Self::CALLBACK_PATH),
            post_logout_redirect_url: format!("{public_origin}/"),
            state_secret: state_secret.into(),
        }
    }
}

impl fmt::Debug for OidcConfig {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("OidcConfig")
            .field("issuer", &self.issuer)
            .field("client_id", &self.client_id)
            .field(
                "client_secret",
                &self.client_secret.as_ref().map(|_| "<redacted>"),
            )
            .field("scopes", &self.scopes)
            .field("name", &self.name)
            .field("admin_group", &self.admin_group)
            .field("groups_claim", &self.groups_claim)
            .field("redirect_url", &self.redirect_url)
            .finish_non_exhaustive()
    }
}

/// Space-separated and deduplicated; always includes `openid`.
fn parse_scopes(value: &str) -> Vec<String> {
    let mut scopes = vec!["openid".to_owned()];
    for scope in value.split_whitespace() {
        if !scopes.iter().any(|s| s == scope) {
            scopes.push(scope.to_owned());
        }
    }
    scopes
}

impl AuthConfig {
    pub const DEFAULT_SESSION_TTL: Duration = Duration::from_hours(30 * 24);
}

impl Default for AuthConfig {
    fn default() -> Self {
        Self {
            local_enabled: true,
            registration_enabled: true,
            require_login: false,
            session_ttl: Self::DEFAULT_SESSION_TTL,
            cookie_secure: true,
            public_origin: None,
            oidc: None,
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct YouTubeConfig {
    /// `rustypipe-botguard` binary; looked up in the working directory and `PATH` when unset.
    pub botguard_bin: Option<PathBuf>,
    /// Persists RustyPipe's client cache and botguard snapshot; in-memory and temp dir when unset.
    pub cache_dir: Option<PathBuf>,
    /// Country whose trending videos are shown.
    pub trending_region: Country,
}

impl Default for YouTubeConfig {
    fn default() -> Self {
        Self {
            botguard_bin: None,
            cache_dir: None,
            trending_region: Country::Us,
        }
    }
}

/// ISO 3166-1 alpha-2 country code known to YouTube.
struct Region(Country);

impl FromStr for Region {
    type Err = String;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        let code = s.trim().to_ascii_uppercase();
        // `ZZ` (global) only exists for music charts.
        match serde_json::from_value::<Country>(serde_json::Value::String(code)) {
            Ok(country) if country != Country::Zz => Ok(Self(country)),
            _ => Err("expected a two-letter country code supported by YouTube, e.g. `US`".into()),
        }
    }
}

#[derive(Clone, PartialEq, Eq)]
pub struct ProxyConfig {
    /// HMAC key for proxy URLs; must be the same on all replicas.
    pub secret: String,
    /// Lifetime of signed stream and subtitle URLs.
    pub stream_ttl: Duration,
    /// Lifetime of signed image URLs.
    pub image_ttl: Duration,
}

impl ProxyConfig {
    pub const DEFAULT_STREAM_TTL: Duration = Duration::from_hours(6);
    pub const DEFAULT_IMAGE_TTL: Duration = Duration::from_hours(7 * 24);

    pub fn new(secret: impl Into<String>) -> Self {
        Self {
            secret: secret.into(),
            stream_ttl: Self::DEFAULT_STREAM_TTL,
            image_ttl: Self::DEFAULT_IMAGE_TTL,
        }
    }
}

impl fmt::Debug for ProxyConfig {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("ProxyConfig")
            .field("secret", &"<redacted>")
            .field("stream_ttl", &self.stream_ttl)
            .field("image_ttl", &self.image_ttl)
            .finish()
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RateLimitConfig {
    /// Sustained requests per second per client IP.
    pub per_second: NonZeroU32,
    pub burst: NonZeroU32,
    /// Header a trusted reverse proxy sets to the client IP. The last entry is used.
    pub trusted_header: Option<HeaderName>,
}

impl Default for RateLimitConfig {
    fn default() -> Self {
        Self {
            per_second: NonZeroU32::new(50).unwrap(),
            burst: NonZeroU32::new(200).unwrap(),
            trusted_header: None,
        }
    }
}

impl fmt::Debug for Config {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("Config")
            .field("bind", &self.bind)
            .field("database_url", &"<redacted>")
            .field("database_max_connections", &self.database_max_connections)
            .field("log_filter", &self.log_filter)
            .field("log_format", &self.log_format)
            .field("youtube", &self.youtube)
            .field("proxy", &self.proxy)
            .field("rate_limit", &self.rate_limit)
            .field("auth", &self.auth)
            .field("feed", &self.feed)
            .field("sponsorblock", &self.sponsorblock)
            .field("history_max_retention", &self.history_max_retention)
            .finish()
    }
}

impl Config {
    pub fn from_env() -> Result<Self, ConfigError> {
        Self::from_lookup(|key| std::env::var(key).ok())
    }

    pub fn from_lookup(lookup: impl Fn(&str) -> Option<String>) -> Result<Self, ConfigError> {
        let get = |key: &str| lookup(key).filter(|v| !v.trim().is_empty());

        let rate_limit_enabled = parse_or(get(RATE_LIMIT), RATE_LIMIT, Toggle(true))?.0;
        let rate_limit = if rate_limit_enabled {
            let defaults = RateLimitConfig::default();
            Some(RateLimitConfig {
                per_second: parse_or(
                    get(RATE_LIMIT_PER_SECOND),
                    RATE_LIMIT_PER_SECOND,
                    defaults.per_second,
                )?,
                burst: parse_or(get(RATE_LIMIT_BURST), RATE_LIMIT_BURST, defaults.burst)?,
                trusted_header: parse_opt(get(TRUSTED_PROXY_HEADER), TRUSTED_PROXY_HEADER)?,
            })
        } else {
            None
        };

        let secret = proxy_secret(get(PROXY_SECRET))?;
        Ok(Self {
            bind: parse_or(get(BIND), BIND, SocketAddr::from(([0, 0, 0, 0], 8080)))?,
            database_url: get(DATABASE_URL).ok_or(ConfigError::Missing(DATABASE_URL))?,
            database_max_connections: parse_or(
                get(DATABASE_MAX_CONNECTIONS),
                DATABASE_MAX_CONNECTIONS,
                10,
            )?,
            log_filter: get(LOG).unwrap_or_else(|| "info".into()),
            log_format: parse_or(get(LOG_FORMAT), LOG_FORMAT, LogFormat::default())?,
            youtube: YouTubeConfig {
                botguard_bin: get(BOTGUARD_BIN).map(PathBuf::from),
                cache_dir: get(YOUTUBE_CACHE_DIR).map(PathBuf::from),
                trending_region: parse_or(
                    get(TRENDING_REGION),
                    TRENDING_REGION,
                    Region(Country::Us),
                )?
                .0,
            },
            proxy: ProxyConfig {
                secret: secret.clone(),
                stream_ttl: seconds_or(
                    get(PROXY_STREAM_TTL),
                    PROXY_STREAM_TTL,
                    ProxyConfig::DEFAULT_STREAM_TTL,
                )?,
                image_ttl: seconds_or(
                    get(PROXY_IMAGE_TTL),
                    PROXY_IMAGE_TTL,
                    ProxyConfig::DEFAULT_IMAGE_TTL,
                )?,
            },
            rate_limit,
            auth: auth_config(&get, &secret)?,
            feed: FeedConfig {
                interval: Duration::from_mins(
                    parse_or(
                        get(FEED_INTERVAL),
                        FEED_INTERVAL,
                        NonZeroU32::new(30).unwrap(),
                    )?
                    .get()
                    .into(),
                ),
            },
            sponsorblock: SponsorBlockConfig {
                enabled: parse_or(
                    get(SPONSORBLOCK_ENABLED),
                    SPONSORBLOCK_ENABLED,
                    Toggle(true),
                )?
                .0,
                api_url: get(SPONSORBLOCK_API_URL).map_or_else(
                    || Ok(SponsorBlockConfig::DEFAULT_API_URL.to_owned()),
                    |value| sponsorblock_api_url(&value),
                )?,
            },
            history_max_retention: parse_or(
                get(HISTORY_MAX_RETENTION),
                HISTORY_MAX_RETENTION,
                Retention::Forever,
            )?,
        })
    }
}

fn auth_config(
    get: &impl Fn(&str) -> Option<String>,
    state_secret: &str,
) -> Result<AuthConfig, ConfigError> {
    let defaults = AuthConfig::default();
    let public_origin = get(PUBLIC_URL)
        .map(|value| public_origin(&value))
        .transpose()?;
    let ttl_days: NonZeroU32 = parse_or(
        get(SESSION_TTL_DAYS),
        SESSION_TTL_DAYS,
        NonZeroU32::new(30).unwrap(),
    )?;
    Ok(AuthConfig {
        local_enabled: parse_or(get(AUTH_LOCAL_ENABLED), AUTH_LOCAL_ENABLED, Toggle(true))?.0,
        registration_enabled: parse_or(
            get(REGISTRATION_ENABLED),
            REGISTRATION_ENABLED,
            Toggle(defaults.registration_enabled),
        )?
        .0,
        require_login: parse_or(
            get(REQUIRE_LOGIN),
            REQUIRE_LOGIN,
            Toggle(defaults.require_login),
        )?
        .0,
        session_ttl: Duration::from_hours(u64::from(ttl_days.get()) * 24),
        cookie_secure: parse_or(get(COOKIE_SECURE), COOKIE_SECURE, Toggle(true))?.0,
        oidc: oidc_config(get, public_origin.as_deref(), state_secret)?,
        public_origin,
    })
}

fn oidc_config(
    get: &impl Fn(&str) -> Option<String>,
    public_origin: Option<&str>,
    state_secret: &str,
) -> Result<Option<OidcConfig>, ConfigError> {
    let (issuer, client_id) = match (get(OIDC_ISSUER), get(OIDC_CLIENT_ID)) {
        (None, None) => return Ok(None),
        (Some(_), None) => {
            return Err(ConfigError::MissingFor {
                var: OIDC_CLIENT_ID,
                because: OIDC_ISSUER,
            });
        }
        (None, Some(_)) => {
            return Err(ConfigError::MissingFor {
                var: OIDC_ISSUER,
                because: OIDC_CLIENT_ID,
            });
        }
        (Some(issuer), Some(client_id)) => (issuer.trim().to_owned(), client_id.trim().to_owned()),
    };
    match reqwest::Url::parse(&issuer) {
        Ok(url) if matches!(url.scheme(), "http" | "https") && url.host_str().is_some() => {}
        _ => {
            return Err(ConfigError::Invalid {
                var: OIDC_ISSUER,
                value: issuer,
                reason: "expected an http or https URL".into(),
            });
        }
    }
    let public_origin = public_origin.ok_or(ConfigError::MissingFor {
        var: PUBLIC_URL,
        because: OIDC_ISSUER,
    })?;
    let mut config = OidcConfig::new(issuer, client_id, public_origin, state_secret);
    config.client_secret = get(OIDC_CLIENT_SECRET);
    if let Some(scopes) = get(OIDC_SCOPES) {
        config.scopes = parse_scopes(&scopes);
    }
    if let Some(name) = get(OIDC_NAME) {
        name.trim().clone_into(&mut config.name);
    }
    config.admin_group = get(OIDC_ADMIN_GROUP).map(|group| group.trim().to_owned());
    if let Some(claim) = get(OIDC_GROUPS_CLAIM) {
        claim.trim().clone_into(&mut config.groups_claim);
    }
    Ok(Some(config))
}

fn public_origin(value: &str) -> Result<String, ConfigError> {
    let invalid = |reason: &str| ConfigError::Invalid {
        var: PUBLIC_URL,
        value: value.to_owned(),
        reason: reason.to_owned(),
    };
    let url = reqwest::Url::parse(value.trim()).map_err(|err| invalid(&err.to_string()))?;
    if !matches!(url.scheme(), "http" | "https") || url.host_str().is_none() {
        return Err(invalid("expected an http or https URL"));
    }
    Ok(url.origin().ascii_serialization())
}

fn sponsorblock_api_url(value: &str) -> Result<String, ConfigError> {
    match reqwest::Url::parse(value.trim()) {
        Ok(url) if matches!(url.scheme(), "http" | "https") && url.host_str().is_some() => {
            Ok(url.as_str().trim_end_matches('/').to_owned())
        }
        _ => Err(ConfigError::Invalid {
            var: SPONSORBLOCK_API_URL,
            value: value.to_owned(),
            reason: "expected an http or https URL".into(),
        }),
    }
}

fn proxy_secret(value: Option<String>) -> Result<String, ConfigError> {
    let secret = value.ok_or(ConfigError::Missing(PROXY_SECRET))?;
    if secret.len() < MIN_PROXY_SECRET_LEN {
        return Err(ConfigError::Invalid {
            var: PROXY_SECRET,
            value: "<redacted>".into(),
            reason: format!("must be at least {MIN_PROXY_SECRET_LEN} characters long"),
        });
    }
    Ok(secret)
}

fn seconds_or(
    value: Option<String>,
    var: &'static str,
    default: Duration,
) -> Result<Duration, ConfigError> {
    let secs: NonZeroU32 = parse_or(
        value,
        var,
        NonZeroU32::new(u32::try_from(default.as_secs()).unwrap_or(u32::MAX)).unwrap(),
    )?;
    Ok(Duration::from_secs(secs.get().into()))
}

struct Toggle(bool);

impl FromStr for Toggle {
    type Err = String;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s.to_ascii_lowercase().as_str() {
            "1" | "true" | "yes" | "on" => Ok(Self(true)),
            "0" | "false" | "no" | "off" => Ok(Self(false)),
            _ => Err("expected `true` or `false`".into()),
        }
    }
}

fn parse_or<T>(value: Option<String>, var: &'static str, default: T) -> Result<T, ConfigError>
where
    T: FromStr,
    T::Err: fmt::Display,
{
    Ok(parse_opt(value, var)?.unwrap_or(default))
}

fn parse_opt<T>(value: Option<String>, var: &'static str) -> Result<Option<T>, ConfigError>
where
    T: FromStr,
    T::Err: fmt::Display,
{
    value
        .map(|value| {
            value
                .trim()
                .parse()
                .map_err(|err: T::Err| ConfigError::Invalid {
                    var,
                    reason: err.to_string(),
                    value,
                })
        })
        .transpose()
}

#[cfg(test)]
mod tests {
    use std::collections::HashMap;

    use super::*;

    const SECRET: &str = "0123456789abcdef0123456789abcdef";

    fn parse(vars: &[(&str, &str)]) -> Result<Config, ConfigError> {
        let map: HashMap<String, String> = vars
            .iter()
            .map(|(k, v)| ((*k).to_owned(), (*v).to_owned()))
            .collect();
        Config::from_lookup(|key| map.get(key).cloned())
    }

    fn required() -> Vec<(&'static str, &'static str)> {
        vec![
            (DATABASE_URL, "postgres://localhost/oxistream"),
            (PROXY_SECRET, SECRET),
        ]
    }

    fn with(extra: &[(&'static str, &'static str)]) -> Result<Config, ConfigError> {
        let mut vars = required();
        vars.extend_from_slice(extra);
        parse(&vars)
    }

    #[test]
    fn defaults() {
        let config = with(&[]).unwrap();
        assert_eq!(config.bind, "0.0.0.0:8080".parse().unwrap());
        assert_eq!(config.database_url, "postgres://localhost/oxistream");
        assert_eq!(config.database_max_connections, 10);
        assert_eq!(config.log_filter, "info");
        assert_eq!(config.log_format, LogFormat::Pretty);
        assert_eq!(config.youtube, YouTubeConfig::default());
        assert_eq!(config.proxy, ProxyConfig::new(SECRET));
        assert_eq!(config.rate_limit, Some(RateLimitConfig::default()));
        assert_eq!(config.auth, AuthConfig::default());
        assert_eq!(config.auth.session_ttl, Duration::from_hours(720));
        assert_eq!(config.feed, FeedConfig::default());
        assert_eq!(config.sponsorblock, SponsorBlockConfig::default());
        assert_eq!(config.history_max_retention, Retention::Forever);
    }

    #[test]
    fn history_max_retention() {
        for (value, expected) in [
            ("30d", Retention::Month),
            (" 1D ", Retention::Day),
            ("forever", Retention::Forever),
            ("365d", Retention::Year),
        ] {
            let config = with(&[(HISTORY_MAX_RETENTION, value)]).unwrap();
            assert_eq!(config.history_max_retention, expected, "{value}");
        }
        for value in ["1y", "2d", "30", "never"] {
            let err = with(&[(HISTORY_MAX_RETENTION, value)]).unwrap_err();
            assert!(
                matches!(
                    err,
                    ConfigError::Invalid {
                        var: HISTORY_MAX_RETENTION,
                        ..
                    }
                ),
                "{err}"
            );
        }
    }

    #[test]
    fn auth_overrides() {
        let config = with(&[
            (AUTH_LOCAL_ENABLED, "false"),
            (REGISTRATION_ENABLED, "off"),
            (REQUIRE_LOGIN, "yes"),
            (SESSION_TTL_DAYS, "7"),
            (COOKIE_SECURE, "0"),
            (PUBLIC_URL, "https://Tube.Example.com:443/some/path"),
        ])
        .unwrap();
        assert_eq!(
            config.auth,
            AuthConfig {
                local_enabled: false,
                registration_enabled: false,
                require_login: true,
                session_ttl: Duration::from_hours(7 * 24),
                cookie_secure: false,
                public_origin: Some("https://tube.example.com".into()),
                oidc: None,
            }
        );
        let config = with(&[(PUBLIC_URL, "http://localhost:5173")]).unwrap();
        assert_eq!(
            config.auth.public_origin.as_deref(),
            Some("http://localhost:5173")
        );
    }

    #[test]
    fn overrides() {
        let config = with(&[
            (DATABASE_URL, "postgres://db/oxistream"),
            (BIND, "127.0.0.1:3000"),
            (DATABASE_MAX_CONNECTIONS, "4"),
            (LOG, "debug,sqlx=warn"),
            (LOG_FORMAT, "JSON"),
            (BOTGUARD_BIN, "/usr/local/bin/rustypipe-botguard"),
            (YOUTUBE_CACHE_DIR, "/var/lib/oxistream"),
            (PROXY_STREAM_TTL, "3600"),
            (PROXY_IMAGE_TTL, "86400"),
            (RATE_LIMIT_PER_SECOND, "5"),
            (RATE_LIMIT_BURST, "10"),
            (TRUSTED_PROXY_HEADER, "X-Forwarded-For"),
            (FEED_INTERVAL, "5"),
        ])
        .unwrap();
        assert_eq!(config.feed.interval, Duration::from_mins(5));
        assert_eq!(config.bind, "127.0.0.1:3000".parse().unwrap());
        assert_eq!(config.database_max_connections, 4);
        assert_eq!(config.log_filter, "debug,sqlx=warn");
        assert_eq!(config.log_format, LogFormat::Json);
        assert_eq!(
            config.youtube.botguard_bin.as_deref(),
            Some("/usr/local/bin/rustypipe-botguard".as_ref())
        );
        assert_eq!(
            config.youtube.cache_dir.as_deref(),
            Some("/var/lib/oxistream".as_ref())
        );
        assert_eq!(config.proxy.stream_ttl, Duration::from_hours(1));
        assert_eq!(config.proxy.image_ttl, Duration::from_hours(24));
        let rate_limit = config.rate_limit.unwrap();
        assert_eq!(rate_limit.per_second.get(), 5);
        assert_eq!(rate_limit.burst.get(), 10);
        assert_eq!(
            rate_limit.trusted_header,
            Some(HeaderName::from_static("x-forwarded-for"))
        );
    }

    #[test]
    fn sponsorblock() {
        let config = with(&[
            (SPONSORBLOCK_ENABLED, "false"),
            (SPONSORBLOCK_API_URL, " https://sb.example.com/ "),
        ])
        .unwrap();
        assert_eq!(
            config.sponsorblock,
            SponsorBlockConfig {
                enabled: false,
                api_url: "https://sb.example.com".into(),
            }
        );
        let config = with(&[(SPONSORBLOCK_API_URL, "http://127.0.0.1:9000/sb/")]).unwrap();
        assert_eq!(config.sponsorblock.api_url, "http://127.0.0.1:9000/sb");
    }

    #[test]
    fn rate_limit_can_be_disabled() {
        let config = with(&[(RATE_LIMIT, "false")]).unwrap();
        assert_eq!(config.rate_limit, None);
    }

    #[test]
    fn missing_required_values() {
        assert_eq!(
            parse(&[(PROXY_SECRET, SECRET)]).unwrap_err(),
            ConfigError::Missing(DATABASE_URL)
        );
        assert_eq!(
            parse(&[(DATABASE_URL, "  "), (PROXY_SECRET, SECRET)]).unwrap_err(),
            ConfigError::Missing(DATABASE_URL)
        );
        assert_eq!(
            parse(&[(DATABASE_URL, "postgres://x")]).unwrap_err(),
            ConfigError::Missing(PROXY_SECRET)
        );
    }

    #[test]
    fn rejects_short_proxy_secret() {
        let err = with(&[(PROXY_SECRET, "too-short")]).unwrap_err();
        assert!(
            matches!(
                err,
                ConfigError::Invalid {
                    var: PROXY_SECRET,
                    ..
                }
            ),
            "{err}"
        );
        assert!(!err.to_string().contains("too-short"));
    }

    #[test]
    fn trending_region() {
        let config = with(&[(TRENDING_REGION, " de ")]).unwrap();
        assert_eq!(config.youtube.trending_region, Country::De);
    }

    #[test]
    fn invalid_values() {
        let err = with(&[(BIND, "nope")]).unwrap_err();
        assert!(
            matches!(err, ConfigError::Invalid { var: BIND, .. }),
            "{err}"
        );

        let err = with(&[(LOG_FORMAT, "xml")]).unwrap_err();
        assert_eq!(
            err.to_string(),
            "OXISTREAM_LOG_FORMAT has invalid value \"xml\": expected `pretty` or `json`"
        );

        for (var, value) in [
            (PROXY_STREAM_TTL, "0"),
            (RATE_LIMIT, "maybe"),
            (RATE_LIMIT_BURST, "0"),
            (TRUSTED_PROXY_HEADER, "not a header"),
            (SESSION_TTL_DAYS, "0"),
            (REQUIRE_LOGIN, "maybe"),
            (PUBLIC_URL, "tube.example.com"),
            (PUBLIC_URL, "ftp://tube.example.com"),
            (FEED_INTERVAL, "0"),
            (TRENDING_REGION, "XX"),
            (TRENDING_REGION, "ZZ"),
            (TRENDING_REGION, "Germany"),
            (SPONSORBLOCK_ENABLED, "maybe"),
            (SPONSORBLOCK_API_URL, "sponsor.ajay.app"),
        ] {
            let err = with(&[(var, value)]).unwrap_err();
            assert!(
                matches!(err, ConfigError::Invalid { var: v, .. } if v == var),
                "{err}"
            );
        }
    }

    #[test]
    fn debug_redacts_secrets() {
        let config = with(&[
            (DATABASE_URL, "postgres://user:secret@db/x"),
            (OIDC_ISSUER, "https://id.example.com/realms/home"),
            (OIDC_CLIENT_ID, "oxistream"),
            (OIDC_CLIENT_SECRET, "client-secret-value"),
            (PUBLIC_URL, "https://tube.example.com"),
        ])
        .unwrap();
        let debug = format!("{config:?}");
        assert!(!debug.contains("secret@"));
        assert!(!debug.contains(SECRET));
        assert!(!debug.contains("client-secret-value"));
    }

    #[test]
    fn oidc_is_enabled_by_issuer_and_client_id() {
        assert_eq!(with(&[]).unwrap().auth.oidc, None);
        let config = with(&[
            (OIDC_ISSUER, "https://id.example.com/realms/home"),
            (OIDC_CLIENT_ID, "oxistream"),
            (PUBLIC_URL, "https://tube.example.com/ignored"),
        ])
        .unwrap();
        let oidc = config.auth.oidc.unwrap();
        assert_eq!(
            oidc,
            OidcConfig::new(
                "https://id.example.com/realms/home",
                "oxistream",
                "https://tube.example.com",
                SECRET
            )
        );
        assert_eq!(oidc.scopes, ["openid", "profile", "email"]);
        assert_eq!(oidc.name, "SSO");
        assert_eq!(oidc.groups_claim, "groups");
        assert_eq!(oidc.admin_group, None);
        assert_eq!(oidc.client_secret, None);
        assert_eq!(
            oidc.redirect_url,
            "https://tube.example.com/api/v0/auth/oidc/callback"
        );
        assert_eq!(oidc.post_logout_redirect_url, "https://tube.example.com/");
    }

    #[test]
    fn oidc_overrides() {
        let oidc = with(&[
            (OIDC_ISSUER, "http://localhost:8180/realms/dev"),
            (OIDC_CLIENT_ID, "tube"),
            (OIDC_CLIENT_SECRET, "s3cret"),
            (OIDC_SCOPES, "profile groups  profile"),
            (OIDC_NAME, " Authentik "),
            (OIDC_ADMIN_GROUP, "/admins"),
            (OIDC_GROUPS_CLAIM, "roles"),
            (PUBLIC_URL, "http://localhost:5173"),
        ])
        .unwrap()
        .auth
        .oidc
        .unwrap();
        assert_eq!(oidc.client_secret.as_deref(), Some("s3cret"));
        assert_eq!(oidc.scopes, ["openid", "profile", "groups"]);
        assert_eq!(oidc.name, "Authentik");
        assert_eq!(oidc.admin_group.as_deref(), Some("/admins"));
        assert_eq!(oidc.groups_claim, "roles");
    }

    #[test]
    fn oidc_needs_complete_configuration() {
        assert_eq!(
            with(&[(OIDC_ISSUER, "https://id.example.com")]).unwrap_err(),
            ConfigError::MissingFor {
                var: OIDC_CLIENT_ID,
                because: OIDC_ISSUER
            }
        );
        assert_eq!(
            with(&[(OIDC_CLIENT_ID, "oxistream")]).unwrap_err(),
            ConfigError::MissingFor {
                var: OIDC_ISSUER,
                because: OIDC_CLIENT_ID
            }
        );
        let err = with(&[
            (OIDC_ISSUER, "https://id.example.com"),
            (OIDC_CLIENT_ID, "oxistream"),
        ])
        .unwrap_err();
        assert_eq!(
            err.to_string(),
            "OXISTREAM_PUBLIC_URL is required when OXISTREAM_OIDC_ISSUER is set"
        );
        let err = with(&[
            (OIDC_ISSUER, "id.example.com"),
            (OIDC_CLIENT_ID, "oxistream"),
            (PUBLIC_URL, "https://tube.example.com"),
        ])
        .unwrap_err();
        assert!(
            matches!(
                err,
                ConfigError::Invalid {
                    var: OIDC_ISSUER,
                    ..
                }
            ),
            "{err}"
        );
    }
}
