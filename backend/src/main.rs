use std::collections::HashMap;
use std::path::PathBuf;
use std::sync::Arc;
use std::time::Duration;

use gantry_board::agent::claude_code::ClaudeCodeExecutor;
use gantry_board::agent::executor::AgentExecutor;
use gantry_board::agent::gemini_cli::GeminiCliExecutor;
use gantry_board::agent::orchestrator::AgentOrchestrator;
use gantry_board::config::Config;
use gantry_board::db;
use gantry_board::models::agent_session::AgentType;
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

    if !config.cookie_secure {
        tracing::warn!("cookie_secure is false — session cookies will be sent over HTTP. Set GANTRY_COOKIE_SECURE=true in production.");
    }

    if config.cors_origin.is_none() {
        tracing::warn!(
            "cors_origin is not set — CORS is permissive. Set GANTRY_CORS_ORIGIN in production."
        );
    }

    let pool = db::init_pool(&config.database_url).await?;
    let sse_hub = Arc::new(SseHub::default());

    let repo_path = config
        .repository_path
        .as_deref()
        .map(PathBuf::from)
        .unwrap_or_else(|| PathBuf::from("."));
    let mut executors: HashMap<AgentType, Arc<dyn AgentExecutor>> = HashMap::new();
    executors.insert(AgentType::ClaudeCode, Arc::new(ClaudeCodeExecutor));
    executors.insert(AgentType::GeminiCli, Arc::new(GeminiCliExecutor));
    let orchestrator = Arc::new(AgentOrchestrator::new(
        executors,
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
    let app = gantry_board::app(state)?;

    // Spawn background task for periodic session cleanup
    let cleanup_interval_secs = config.session_cleanup_interval_secs.max(1);
    let cleanup_interval = Duration::from_secs(cleanup_interval_secs);
    tokio::spawn(async move {
        let mut interval = tokio::time::interval(cleanup_interval);
        interval.set_missed_tick_behavior(tokio::time::MissedTickBehavior::Delay);
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
    axum::serve(
        listener,
        app.into_make_service_with_connect_info::<std::net::SocketAddr>(),
    )
    .await?;

    Ok(())
}
