use std::path::PathBuf;
use std::sync::Arc;
use std::time::Duration;

use gantry_board::agent::claude_code::ClaudeCodeExecutor;
use gantry_board::agent::orchestrator::AgentOrchestrator;
use gantry_board::config::Config;
use gantry_board::db;
use gantry_board::services::session_service;
use gantry_board::sse::hub::SseHub;
use gantry_board::AppState;
use tracing_subscriber::EnvFilter;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    tracing_subscriber::fmt()
        .with_env_filter(EnvFilter::try_from_default_env().unwrap_or_else(|_| "info".into()))
        .init();

    let config = Config::load()?;

    #[cfg(debug_assertions)]
    if config.auth_disabled {
        tracing::warn!("Authentication is DISABLED. Do not use in production!");
    }

    let pool = db::init_pool(&config.database_url).await?;
    let sse_hub = Arc::new(SseHub::default());

    let repo_path = config
        .repository_path
        .as_deref()
        .map(PathBuf::from)
        .unwrap_or_else(|| PathBuf::from("."));
    let executor = Arc::new(ClaudeCodeExecutor);
    let orchestrator = Arc::new(AgentOrchestrator::new(
        executor,
        pool.clone(),
        repo_path,
        Arc::clone(&sse_hub),
    ));

    let cleanup_pool = pool.clone();

    let state = AppState {
        pool,
        sse_hub,
        config: Arc::new(config.clone()),
        orchestrator,
    };
    let app = gantry_board::app(state);

    // Spawn background task for periodic session cleanup
    let cleanup_interval = Duration::from_secs(config.session_cleanup_interval_secs);
    tokio::spawn(async move {
        let mut interval = tokio::time::interval(cleanup_interval);
        interval.tick().await; // skip the immediate first tick
        loop {
            interval.tick().await;
            match session_service::cleanup_expired_sessions(&cleanup_pool).await {
                Ok(count) if count > 0 => {
                    tracing::info!(count, "cleaned up expired sessions");
                }
                Err(e) => {
                    tracing::error!(error = %e, "failed to cleanup expired sessions");
                }
                _ => {}
            }
        }
    });

    let listener = tokio::net::TcpListener::bind(&config.bind_addr).await?;
    tracing::info!("listening on {}", config.bind_addr);
    axum::serve(listener, app).await?;

    Ok(())
}
