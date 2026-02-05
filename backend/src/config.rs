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

    /// Disable authentication (for development only)
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
            auth_disabled: false,
            cookie_secure: false,
        }
    }
}
