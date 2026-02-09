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

    /// Use secure cookies (HTTPS only) - should be true in production
    #[serde(default)]
    pub cookie_secure: bool,
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

impl Config {
    pub fn load() -> Result<Self, Box<figment::Error>> {
        dotenvy::dotenv().ok();

        Ok(Figment::new()
            .merge(Toml::file("config.toml"))
            .merge(Env::prefixed("GANTRY_"))
            .extract()?)
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
            cookie_secure: false,
        }
    }
}

#[cfg(test)]
#[cfg(debug_assertions)]
mod tests {
    use super::*;

    #[test]
    fn test_auth_disabled_defaults_to_false() {
        let config = Config::default();
        assert!(!config.auth_disabled);
    }

    #[test]
    fn test_auth_disabled_can_be_enabled_in_debug_builds() {
        let config = Config {
            auth_disabled: true,
            ..Default::default()
        };
        assert!(config.auth_disabled);
    }
}
