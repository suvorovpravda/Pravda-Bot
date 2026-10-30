//! Startup configuration: process environment only; no implicit .env loading.
use std::{fmt, time::Duration};

#[derive(Debug, Clone)]
pub struct ServerConfig {
    pub port: u16,
    pub database_path: String,
    pub stale_after: Duration,
    pub max_skew: Duration,
    pub max_spread_percent: f64,
    pub market_feeds_enabled: bool,
}
#[derive(Debug, PartialEq, Eq)]
pub struct ConfigError(&'static str);
impl fmt::Display for ConfigError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "Invalid configuration: {}", self.0)
    }
}
impl std::error::Error for ConfigError {}
impl Default for ServerConfig {
    fn default() -> Self {
        Self {
            port: 8080,
            database_path: "pravda.db".into(),
            stale_after: Duration::from_secs(4),
            max_skew: Duration::from_secs(2),
            max_spread_percent: 5.0,
            market_feeds_enabled: true,
        }
    }
}
impl ServerConfig {
    pub fn from_env() -> Result<Self, ConfigError> {
        Self::from_lookup(|key| match std::env::var(key) {
            Ok(value) => Ok(Some(value)),
            Err(std::env::VarError::NotPresent) => Ok(None),
            Err(_) => Err(ConfigError(key)),
        })
    }
    fn from_lookup(
        mut read: impl FnMut(&'static str) -> Result<Option<String>, ConfigError>,
    ) -> Result<Self, ConfigError> {
        let mut config = Self::default();
        if let Some(raw) = read("PRAVDA_MARKET_FEEDS_ENABLED")? {
            config.market_feeds_enabled = match raw.as_str() {
                "true" => true,
                "false" => false,
                _ => return Err(ConfigError("PRAVDA_MARKET_FEEDS_ENABLED")),
            };
        }
        if let Some(raw) = read("DASHBOARD_PORT")? {
            config.port = raw
                .parse::<u16>()
                .map_err(|_| ConfigError("DASHBOARD_PORT"))?;
            if config.port == 0 {
                return Err(ConfigError("DASHBOARD_PORT"));
            }
        }
        if let Some(path) = read("PRAVDA_DB_PATH")? {
            if path.trim().is_empty() || path.contains('\0') || path == ":memory:" {
                return Err(ConfigError("PRAVDA_DB_PATH"));
            }
            config.database_path = path;
        }
        for (name, destination, allow_zero) in [
            ("MARKET_DATA_STALE_SECONDS", &mut config.stale_after, false),
            ("MAX_EXCHANGE_SKEW_SECONDS", &mut config.max_skew, true),
        ] {
            if let Some(raw) = read(name)? {
                let seconds = raw.parse::<f64>().map_err(|_| ConfigError(name))?;
                if !seconds.is_finite()
                    || seconds < 0.0
                    || (!allow_zero && seconds == 0.0)
                    || seconds > 3600.0
                {
                    return Err(ConfigError(name));
                }
                *destination =
                    Duration::try_from_secs_f64(seconds).map_err(|_| ConfigError(name))?;
                if !allow_zero && destination.is_zero() {
                    return Err(ConfigError(name));
                }
            }
        }
        if let Some(raw) = read("MAX_ALLOWED_SPREAD_PERCENT")? {
            let value = raw
                .parse::<f64>()
                .map_err(|_| ConfigError("MAX_ALLOWED_SPREAD_PERCENT"))?;
            if !value.is_finite() || value <= 0.0 || value > 100.0 {
                return Err(ConfigError("MAX_ALLOWED_SPREAD_PERCENT"));
            }
            config.max_spread_percent = value;
        }
        Ok(config)
    }
}
#[cfg(test)]
mod tests {
    use super::*;
    fn parse(key: &'static str, value: &str) -> Result<ServerConfig, ConfigError> {
        ServerConfig::from_lookup(|name| Ok((name == key).then(|| value.to_owned())))
    }
    #[test]
    fn defaults_and_overrides_are_explicit() {
        assert_eq!(ServerConfig::from_lookup(|_| Ok(None)).unwrap().port, 8080);
        assert!(
            ServerConfig::from_lookup(|_| Ok(None))
                .unwrap()
                .market_feeds_enabled
        );
        assert!(
            !parse("PRAVDA_MARKET_FEEDS_ENABLED", "false")
                .unwrap()
                .market_feeds_enabled
        );
        assert_eq!(parse("DASHBOARD_PORT", "8081").unwrap().port, 8081);
        assert_eq!(
            parse("MAX_EXCHANGE_SKEW_SECONDS", "0").unwrap().max_skew,
            Duration::ZERO
        );
        assert_eq!(
            parse("MARKET_DATA_STALE_SECONDS", "1.5")
                .unwrap()
                .stale_after,
            Duration::from_millis(1500)
        );
        assert_eq!(
            parse("PRAVDA_DB_PATH", "data/local.db")
                .unwrap()
                .database_path,
            "data/local.db"
        );
    }
    #[test]
    fn invalid_configuration_fails_without_echoing_values() {
        for (key, values) in [
            ("DASHBOARD_PORT", vec!["0", "65536", "abc", "-1"]),
            ("PRAVDA_MARKET_FEEDS_ENABLED", vec!["", "0", "TRUE"]),
            ("PRAVDA_DB_PATH", vec!["", " ", ":memory:", "secret\0path"]),
            (
                "MARKET_DATA_STALE_SECONDS",
                vec!["NaN", "inf", "0", "-1", "1e-100", "3601"],
            ),
            ("MAX_ALLOWED_SPREAD_PERCENT", vec!["NaN", "0", "-1", "101"]),
        ] {
            for value in values {
                assert_eq!(
                    parse(key, value).unwrap_err().to_string(),
                    format!("Invalid configuration: {key}")
                );
            }
        }
    }
}
