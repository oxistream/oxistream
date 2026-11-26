use std::fmt;
use std::net::SocketAddr;
use std::str::FromStr;

const BIND: &str = "OXISTREAM_BIND";
const DATABASE_URL: &str = "OXISTREAM_DATABASE_URL";
const DATABASE_MAX_CONNECTIONS: &str = "OXISTREAM_DATABASE_MAX_CONNECTIONS";
const LOG: &str = "OXISTREAM_LOG";
const LOG_FORMAT: &str = "OXISTREAM_LOG_FORMAT";

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
}

impl fmt::Debug for Config {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("Config")
            .field("bind", &self.bind)
            .field("database_url", &"<redacted>")
            .field("database_max_connections", &self.database_max_connections)
            .field("log_filter", &self.log_filter)
            .field("log_format", &self.log_format)
            .finish()
    }
}

impl Config {
    pub fn from_env() -> Result<Self, ConfigError> {
        Self::from_lookup(|key| std::env::var(key).ok())
    }

    pub fn from_lookup(lookup: impl Fn(&str) -> Option<String>) -> Result<Self, ConfigError> {
        let get = |key: &str| lookup(key).filter(|v| !v.trim().is_empty());

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
        })
    }
}

fn parse_or<T>(value: Option<String>, var: &'static str, default: T) -> Result<T, ConfigError>
where
    T: FromStr,
    T::Err: fmt::Display,
{
    match value {
        None => Ok(default),
        Some(value) => value.parse().map_err(|err: T::Err| ConfigError::Invalid {
            var,
            reason: err.to_string(),
            value,
        }),
    }
}

#[cfg(test)]
mod tests {
    use std::collections::HashMap;

    use super::*;

    fn parse(vars: &[(&str, &str)]) -> Result<Config, ConfigError> {
        let map: HashMap<String, String> = vars
            .iter()
            .map(|(k, v)| ((*k).to_owned(), (*v).to_owned()))
            .collect();
        Config::from_lookup(|key| map.get(key).cloned())
    }

    #[test]
    fn defaults() {
        let config = parse(&[(DATABASE_URL, "postgres://localhost/oxistream")]).unwrap();
        assert_eq!(config.bind, "0.0.0.0:8080".parse().unwrap());
        assert_eq!(config.database_url, "postgres://localhost/oxistream");
        assert_eq!(config.database_max_connections, 10);
        assert_eq!(config.log_filter, "info");
        assert_eq!(config.log_format, LogFormat::Pretty);
    }

    #[test]
    fn overrides() {
        let config = parse(&[
            (DATABASE_URL, "postgres://db/oxistream"),
            (BIND, "127.0.0.1:3000"),
            (DATABASE_MAX_CONNECTIONS, "4"),
            (LOG, "debug,sqlx=warn"),
            (LOG_FORMAT, "JSON"),
        ])
        .unwrap();
        assert_eq!(config.bind, "127.0.0.1:3000".parse().unwrap());
        assert_eq!(config.database_max_connections, 4);
        assert_eq!(config.log_filter, "debug,sqlx=warn");
        assert_eq!(config.log_format, LogFormat::Json);
    }

    #[test]
    fn missing_database_url() {
        assert_eq!(parse(&[]).unwrap_err(), ConfigError::Missing(DATABASE_URL));
        assert_eq!(
            parse(&[(DATABASE_URL, "  ")]).unwrap_err(),
            ConfigError::Missing(DATABASE_URL)
        );
    }

    #[test]
    fn invalid_values() {
        let err = parse(&[(DATABASE_URL, "postgres://x"), (BIND, "nope")]).unwrap_err();
        assert!(
            matches!(err, ConfigError::Invalid { var: BIND, .. }),
            "{err}"
        );

        let err = parse(&[(DATABASE_URL, "postgres://x"), (LOG_FORMAT, "xml")]).unwrap_err();
        assert_eq!(
            err.to_string(),
            "OXISTREAM_LOG_FORMAT has invalid value \"xml\": expected `pretty` or `json`"
        );
    }

    #[test]
    fn debug_redacts_database_url() {
        let config = parse(&[(DATABASE_URL, "postgres://user:secret@db/x")]).unwrap();
        assert!(!format!("{config:?}").contains("secret"));
    }
}
