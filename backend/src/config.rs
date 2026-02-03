use figment::providers::{Env, Format, Toml};
use figment::Figment;
use serde::Deserialize;

#[derive(Debug, Deserialize, Clone)]
pub struct Config {
    #[serde(default = "default_bind_addr")]
    pub bind_addr: String,

    #[serde(default = "default_database_url")]
    pub database_url: String,
}

fn default_bind_addr() -> String {
    "0.0.0.0:3000".to_string()
}

fn default_database_url() -> String {
    "sqlite:gantry_board.db?mode=rwc".to_string()
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
