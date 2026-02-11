use axum::http::HeaderValue;
use figment::providers::{Env, Format, Toml};
use figment::Figment;
use serde::Deserialize;

#[derive(Debug, Deserialize, Clone)]
pub struct Config {
    #[serde(default = "default_bind_addr")]
    pub bind_addr: String,

    #[serde(default = "default_database_url")]
    pub database_url: String,

    /// Session duration in hours (default: 168 = 1 week)
    #[serde(default = "default_session_duration_hours")]
    pub session_duration_hours: u64,

    /// Disable authentication (debug builds only — never available in release)
    #[cfg(debug_assertions)]
    #[serde(default)]
    pub auth_disabled: bool,

    /// Use secure cookies (HTTPS only) - defaults to true; set to false only for local HTTP dev
    #[serde(default = "default_cookie_secure")]
    pub cookie_secure: bool,

    /// Path to the git repository for worktree management
    #[serde(default)]
    pub repository_path: Option<String>,

    /// Interval in seconds between session cleanup runs (default: 3600 = 1 hour)
    #[serde(default = "default_session_cleanup_interval_secs")]
    pub session_cleanup_interval_secs: u64,

    /// Allowed CORS origin (e.g. "http://localhost:5173"). When unset, CORS is permissive.
    #[serde(default)]
    pub cors_origin: Option<String>,
}

fn default_bind_addr() -> String {
    "0.0.0.0:3000".to_string()
}

fn default_database_url() -> String {
    "sqlite:gantry_board.db?mode=rwc".to_string()
}

fn default_session_duration_hours() -> u64 {
    168 // 1 week
}

fn default_cookie_secure() -> bool {
    true
}

fn default_session_cleanup_interval_secs() -> u64 {
    3600 // 1 hour
}

impl Config {
    pub fn load() -> Result<Self, Box<figment::Error>> {
        dotenvy::dotenv().ok();

        let config: Self = Figment::new()
            .merge(Toml::file("config.toml"))
            .merge(Env::prefixed("GANTRY_"))
            .extract()?;

        config.validate();

        Ok(config)
    }

    /// Validate config values that cannot be expressed via serde alone.
    pub fn validate(&self) {
        if let Some(origin) = &self.cors_origin {
            origin.parse::<HeaderValue>().unwrap_or_else(|_| {
                panic!("GANTRY_CORS_ORIGIN is not a valid HTTP header value: {origin:?}")
            });
        }

        // In release builds, CORS origin must be explicitly configured
        #[cfg(not(debug_assertions))]
        if self.cors_origin.is_none() {
            panic!(
                "GANTRY_CORS_ORIGIN must be set in production. \
                 Permissive CORS is only allowed in debug builds."
            );
        }
    }

    /// Parse `cors_origin` into an `HeaderValue`, returning `None` when unset.
    pub fn cors_origin_header(&self) -> Option<HeaderValue> {
        self.cors_origin
            .as_ref()
            .map(|o| o.parse().expect("cors_origin already validated"))
    }
}

impl Default for Config {
    fn default() -> Self {
        Self {
            bind_addr: default_bind_addr(),
            database_url: default_database_url(),
            session_duration_hours: default_session_duration_hours(),
            #[cfg(debug_assertions)]
            auth_disabled: false,
            cookie_secure: default_cookie_secure(),
            repository_path: None,
            session_cleanup_interval_secs: default_session_cleanup_interval_secs(),
            cors_origin: None,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[cfg(debug_assertions)]
    #[test]
    fn test_auth_disabled_defaults_to_false() {
        let config = Config::default();
        assert!(!config.auth_disabled);
    }

    #[cfg(debug_assertions)]
    #[test]
    fn test_auth_disabled_can_be_enabled_in_debug_builds() {
        let config = Config {
            auth_disabled: true,
            ..Default::default()
        };
        assert!(config.auth_disabled);
    }

    #[test]
    fn test_session_cleanup_interval_defaults_to_3600() {
        let config = Config::default();
        assert_eq!(config.session_cleanup_interval_secs, 3600);
    }

    #[test]
    fn test_cors_origin_header_returns_none_when_unset() {
        let config = Config::default();
        assert!(config.cors_origin_header().is_none());
    }

    #[test]
    fn test_cors_origin_header_returns_value_when_set() {
        let config = Config {
            cors_origin: Some("http://localhost:5173".to_string()),
            ..Default::default()
        };
        let header = config.cors_origin_header().expect("should return header");
        assert_eq!(header.to_str().unwrap(), "http://localhost:5173");
    }

    #[test]
    #[should_panic(expected = "not a valid HTTP header value")]
    fn test_validate_rejects_invalid_cors_origin() {
        let config = Config {
            cors_origin: Some("not a valid \x00 header".to_string()),
            ..Default::default()
        };
        config.validate();
    }

    #[test]
    fn test_cookie_secure_defaults_to_true() {
        let config = Config::default();
        assert!(config.cookie_secure);
    }

    #[test]
    fn test_validate_accepts_cors_origin_set() {
        let config = Config {
            cors_origin: Some("http://localhost:5173".to_string()),
            ..Default::default()
        };
        config.validate(); // should not panic
    }

    #[cfg(debug_assertions)]
    #[test]
    fn test_validate_accepts_missing_cors_in_debug_build() {
        let config = Config {
            cors_origin: None,
            ..Default::default()
        };
        config.validate(); // should not panic in debug builds
    }
}
