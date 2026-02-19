use axum::http::HeaderValue;
use figment::providers::{Env, Format, Toml};
use figment::Figment;
use serde::Deserialize;

#[derive(Debug, thiserror::Error)]
pub enum ConfigError {
    #[error("GANTRY_CORS_ORIGIN is not a valid HTTP header value: {0:?}")]
    InvalidCorsOrigin(String),
    #[error(
        "GANTRY_CORS_ORIGIN must be set in production. Permissive CORS is only allowed in debug builds."
    )]
    MissingCorsOriginInRelease,
    #[error("rate limiter configuration error: {0}")]
    RateLimiter(String),
}

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

    /// Maximum database connection pool size (default: 20)
    #[serde(default = "default_max_db_connections")]
    pub max_db_connections: u32,

    /// SSE broadcast channel capacity (default: 4096)
    #[serde(default = "default_sse_broadcast_capacity")]
    pub sse_broadcast_capacity: usize,

    /// Retention period in days for agent session outputs (default: 30)
    #[serde(default = "default_output_retention_days")]
    pub output_retention_days: u64,

    /// HTTP request timeout in seconds (default: 60)
    #[serde(default = "default_request_timeout_secs")]
    pub request_timeout_secs: u64,

    /// Log format: "pretty" (default) or "json"
    #[serde(default = "default_log_format")]
    pub log_format: String,

    /// Docker daemon socket (default: "unix:///var/run/docker.sock")
    #[serde(default = "default_docker_host")]
    pub docker_host: String,

    /// Preview port range start (default: 8100)
    #[serde(default = "default_preview_port_range_start")]
    pub preview_port_range_start: u16,

    /// Preview port range end (default: 8199)
    #[serde(default = "default_preview_port_range_end")]
    pub preview_port_range_end: u16,

    /// Preview base URL (default: "http://localhost")
    #[serde(default = "default_preview_base_url")]
    pub preview_base_url: String,

    /// GitHub Personal Access Token (optional, env: GANTRY_GITHUB_TOKEN)
    #[serde(default)]
    pub github_token: Option<String>,

    /// GitHub sync interval in seconds (default: 300 = 5 minutes)
    #[serde(default = "default_github_sync_interval_secs")]
    pub github_sync_interval_secs: u64,

    /// Allowed Host header values (e.g. ["localhost:3000", "gantry.example.com"]).
    /// When empty, Host header validation is skipped.
    #[serde(default)]
    pub allowed_hosts: Vec<String>,

    /// GitHub Webhook secret for signature verification (env: GANTRY_GITHUB_WEBHOOK_SECRET)
    #[serde(default)]
    pub github_webhook_secret: Option<String>,
}

fn default_bind_addr() -> String {
    "0.0.0.0:3000".to_string()
}

fn default_database_url() -> String {
    "sqlite:./data/gantry_board.db?mode=rwc".to_string()
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

fn default_max_db_connections() -> u32 {
    20
}

fn default_sse_broadcast_capacity() -> usize {
    4096
}

fn default_output_retention_days() -> u64 {
    30
}

fn default_request_timeout_secs() -> u64 {
    60
}

fn default_log_format() -> String {
    "pretty".to_string()
}

fn default_docker_host() -> String {
    "unix:///var/run/docker.sock".to_string()
}

fn default_preview_port_range_start() -> u16 {
    8100
}

fn default_preview_port_range_end() -> u16 {
    8199
}

fn default_preview_base_url() -> String {
    "http://localhost".to_string()
}

fn default_github_sync_interval_secs() -> u64 {
    300 // 5 minutes
}

impl Config {
    pub fn load() -> Result<Self, Box<figment::Error>> {
        dotenvy::dotenv().ok();

        let config: Self = Figment::new()
            .merge(Toml::file("config.toml"))
            .merge(Env::prefixed("GANTRY_"))
            .extract()?;

        config
            .validate()
            .map_err(|e| figment::Error::from(e.to_string()))?;

        Ok(config)
    }

    /// Validate config values that cannot be expressed via serde alone.
    pub fn validate(&self) -> Result<(), ConfigError> {
        if let Some(origin) = &self.cors_origin {
            origin
                .parse::<HeaderValue>()
                .map_err(|_| ConfigError::InvalidCorsOrigin(origin.clone()))?;
        }

        // In release builds, CORS origin must be explicitly configured
        #[cfg(not(debug_assertions))]
        if self.cors_origin.is_none() {
            return Err(ConfigError::MissingCorsOriginInRelease);
        }

        // Warn when auth is enabled but CORS origin is not configured (debug builds)
        #[cfg(debug_assertions)]
        if !self.auth_disabled && self.cors_origin.is_none() {
            tracing::warn!(
                "CORS origin not configured with authentication enabled. \
                 This is a security risk in production deployments. \
                 Set GANTRY_CORS_ORIGIN to your frontend URL."
            );
        }

        // Reject wildcard CORS origin
        if let Some(ref origin) = self.cors_origin {
            if origin == "*" {
                return Err(ConfigError::InvalidCorsOrigin(
                    "wildcard '*' is not allowed; specify an explicit origin".to_string(),
                ));
            }
        }

        Ok(())
    }

    /// Return the repository path for worktree management.
    pub fn repo_path(&self) -> std::path::PathBuf {
        self.repository_path
            .as_deref()
            .map(std::path::PathBuf::from)
            .unwrap_or_else(|| std::path::PathBuf::from("."))
    }

    /// Parse `cors_origin` into an `HeaderValue`, returning `None` when unset.
    pub fn cors_origin_header(&self) -> Result<Option<HeaderValue>, ConfigError> {
        match &self.cors_origin {
            Some(o) => o
                .parse()
                .map(Some)
                .map_err(|_| ConfigError::InvalidCorsOrigin(o.clone())),
            None => Ok(None),
        }
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
            max_db_connections: default_max_db_connections(),
            sse_broadcast_capacity: default_sse_broadcast_capacity(),
            output_retention_days: default_output_retention_days(),
            request_timeout_secs: default_request_timeout_secs(),
            log_format: default_log_format(),
            docker_host: default_docker_host(),
            preview_port_range_start: default_preview_port_range_start(),
            preview_port_range_end: default_preview_port_range_end(),
            preview_base_url: default_preview_base_url(),
            github_token: None,
            github_sync_interval_secs: default_github_sync_interval_secs(),
            allowed_hosts: Vec::new(),
            github_webhook_secret: None,
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
        assert!(config.cors_origin_header().unwrap().is_none());
    }

    #[test]
    fn test_cors_origin_header_returns_value_when_set() {
        let config = Config {
            cors_origin: Some("http://localhost:5173".to_string()),
            ..Default::default()
        };
        let header = config
            .cors_origin_header()
            .expect("should not error")
            .expect("should return Some");
        assert_eq!(header.to_str().unwrap(), "http://localhost:5173");
    }

    #[test]
    fn test_validate_rejects_invalid_cors_origin() {
        let config = Config {
            cors_origin: Some("not a valid \x00 header".to_string()),
            ..Default::default()
        };
        assert!(config.validate().is_err());
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
        assert!(config.validate().is_ok());
    }

    #[cfg(debug_assertions)]
    #[test]
    fn test_validate_accepts_missing_cors_in_debug_build() {
        let config = Config {
            cors_origin: None,
            ..Default::default()
        };
        assert!(config.validate().is_ok());
    }

    #[test]
    fn test_max_db_connections_defaults_to_20() {
        let config = Config::default();
        assert_eq!(config.max_db_connections, 20);
    }

    #[test]
    fn test_sse_broadcast_capacity_defaults_to_4096() {
        let config = Config::default();
        assert_eq!(config.sse_broadcast_capacity, 4096);
    }

    #[test]
    fn test_output_retention_days_defaults_to_30() {
        let config = Config::default();
        assert_eq!(config.output_retention_days, 30);
    }

    #[test]
    fn test_request_timeout_defaults_to_60() {
        let config = Config::default();
        assert_eq!(config.request_timeout_secs, 60);
    }

    #[test]
    fn test_validate_rejects_wildcard_cors_origin() {
        let config = Config {
            cors_origin: Some("*".to_string()),
            ..Default::default()
        };
        let err = config.validate().unwrap_err();
        assert!(err.to_string().contains("wildcard"));
    }

    #[test]
    fn test_allowed_hosts_defaults_to_empty() {
        let config = Config::default();
        assert!(config.allowed_hosts.is_empty());
    }
}
