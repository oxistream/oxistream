use std::fmt;
use std::net::SocketAddr;
use std::num::NonZeroU32;
use std::path::PathBuf;
use std::str::FromStr;
use std::time::Duration;

use axum::http::HeaderName;

const BIND: &str = "OXISTREAM_BIND";
const DATABASE_URL: &str = "OXISTREAM_DATABASE_URL";
const DATABASE_MAX_CONNECTIONS: &str = "OXISTREAM_DATABASE_MAX_CONNECTIONS";
const LOG: &str = "OXISTREAM_LOG";
const LOG_FORMAT: &str = "OXISTREAM_LOG_FORMAT";
const BOTGUARD_BIN: &str = "OXISTREAM_BOTGUARD_BIN";
const YOUTUBE_CACHE_DIR: &str = "OXISTREAM_YOUTUBE_CACHE_DIR";
const PROXY_SECRET: &str = "OXISTREAM_PROXY_SECRET";
const PROXY_STREAM_TTL: &str = "OXISTREAM_PROXY_STREAM_TTL_SECS";
const PROXY_IMAGE_TTL: &str = "OXISTREAM_PROXY_IMAGE_TTL_SECS";
const RATE_LIMIT: &str = "OXISTREAM_RATE_LIMIT";
const RATE_LIMIT_PER_SECOND: &str = "OXISTREAM_RATE_LIMIT_PER_SECOND";
const RATE_LIMIT_BURST: &str = "OXISTREAM_RATE_LIMIT_BURST";
const TRUSTED_PROXY_HEADER: &str = "OXISTREAM_TRUSTED_PROXY_HEADER";

pub const MIN_PROXY_SECRET_LEN: usize = 32;

#[derive(Debug, thiserror::Error, PartialEq, Eq)]
pub enum ConfigError {
    #[error("{0} is required but not set")]
    Missing(&'static str),
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
}

#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct YouTubeConfig {
    /// `rustypipe-botguard` binary; looked up in the working directory and `PATH` when unset.
    pub botguard_bin: Option<PathBuf>,
    /// Persists RustyPipe's client cache and botguard snapshot; in-memory and temp dir when unset.
    pub cache_dir: Option<PathBuf>,
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
            },
            proxy: ProxyConfig {
                secret: proxy_secret(get(PROXY_SECRET))?,
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
        })
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
        ])
        .unwrap();
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
        let config = with(&[(DATABASE_URL, "postgres://user:secret@db/x")]).unwrap();
        let debug = format!("{config:?}");
        assert!(!debug.contains("secret@"));
        assert!(!debug.contains(SECRET));
    }
}
