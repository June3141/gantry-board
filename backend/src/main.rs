use std::sync::Arc;

use gantry_board::config::Config;
use gantry_board::db;
use gantry_board::sse::hub::SseHub;
use gantry_board::ws::hub::Hub;
use gantry_board::AppState;
use tracing_subscriber::EnvFilter;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    tracing_subscriber::fmt()
        .with_env_filter(EnvFilter::try_from_default_env().unwrap_or_else(|_| "info".into()))
        .init();

    let config = Config::load()?;
    let pool = db::init_pool(&config.database_url).await?;

    let state = AppState {
        pool,
        ws_hub: Arc::new(Hub::default()),
        sse_hub: Arc::new(SseHub::default()),
        config: Arc::new(config.clone()),
    };

    let app = gantry_board::app(state);

    let listener = tokio::net::TcpListener::bind(&config.bind_addr).await?;
    tracing::info!("listening on {}", config.bind_addr);
    axum::serve(listener, app).await?;

    Ok(())
}
