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
use gantry_board::services::preview_service::PreviewManager;
use gantry_board::services::{agent_session_output_service, session_service};
use gantry_board::sse::hub::SseHub;
use gantry_board::AppState;
use tokio_util::sync::CancellationToken;
use tracing_subscriber::EnvFilter;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let config = Config::load()?;

    let filter = EnvFilter::try_from_default_env().unwrap_or_else(|_| "info".into());
    match config.log_format.as_str() {
        "json" => tracing_subscriber::fmt()
            .json()
            .with_env_filter(filter)
            .init(),
        _ => tracing_subscriber::fmt().with_env_filter(filter).init(),
    }

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

    let pool = db::init_pool(&config.database_url, config.max_db_connections).await?;
    let sse_hub = Arc::new(SseHub::new(config.sse_broadcast_capacity));

    let repo_path = config
        .repository_path
        .as_deref()
        .map(PathBuf::from)
        .unwrap_or_else(|| PathBuf::from("."));
    let output_buffer = Arc::new(
        gantry_board::services::agent_session_output_service::OutputBuffer::new(pool.clone()),
    );

    let mut executors: HashMap<AgentType, Arc<dyn AgentExecutor>> = HashMap::new();
    executors.insert(AgentType::ClaudeCode, Arc::new(ClaudeCodeExecutor));
    executors.insert(AgentType::GeminiCli, Arc::new(GeminiCliExecutor));
    let orchestrator = Arc::new(AgentOrchestrator::new(
        executors,
        pool.clone(),
        repo_path.clone(),
        Arc::clone(&sse_hub),
        Arc::clone(&output_buffer),
    ));

    let cleanup_pool = pool.clone();
    let cleanup_orchestrator = Arc::clone(&orchestrator);
    let output_retention_days = config.output_retention_days;

    let preview_manager = match PreviewManager::new(
        Arc::new(config.clone()),
        pool.clone(),
        Arc::clone(&sse_hub),
        repo_path,
    ) {
        Ok(pm) => {
            tracing::info!("Docker preview manager initialized");
            Some(Arc::new(pm))
        }
        Err(e) => {
            tracing::warn!(%e, "Docker preview manager unavailable — preview features disabled");
            None
        }
    };

    let github_client = match &config.github_token {
        Some(token) if !token.is_empty() => {
            match gantry_board::github::octocrab_client::OctocrabClient::new(token) {
                Ok(client) => {
                    let inner = Arc::new(client) as Arc<dyn gantry_board::github::api::GitHubApi>;
                    let cached =
                        gantry_board::github::octocrab_client::CachedGitHubClient::new(inner);
                    tracing::info!("GitHub integration initialized (with API cache)");
                    Some(Arc::new(cached) as Arc<dyn gantry_board::github::api::GitHubApi>)
                }
                Err(e) => {
                    tracing::warn!(%e, "GitHub client initialization failed — GitHub features disabled");
                    None
                }
            }
        }
        _ => {
            tracing::info!("GANTRY_GITHUB_TOKEN not set — GitHub features disabled");
            None
        }
    };

    let shutdown_orchestrator = Arc::clone(&orchestrator);

    // Clone values needed by background tasks before moving into AppState
    let sync_github_client = github_client.clone();
    let sync_pool = pool.clone();
    let sync_sse_hub = Arc::clone(&sse_hub);

    let state = AppState {
        pool,
        sse_hub,
        config: Arc::new(config.clone()),
        orchestrator,
        preview_manager,
        github_client,
        output_buffer: Arc::clone(&output_buffer),
        connection_counter: Arc::new(std::sync::atomic::AtomicUsize::new(0)),
        started_at: std::time::Instant::now(),
    };
    let app = gantry_board::app(state)?;

    // Cancellation token for graceful shutdown of background tasks
    let shutdown_token = CancellationToken::new();

    // Spawn periodic output buffer flush
    output_buffer.spawn_periodic_flush(shutdown_token.clone());

    // Spawn background task for periodic session cleanup
    let cleanup_interval_secs = config.session_cleanup_interval_secs.max(1);
    let cleanup_interval = Duration::from_secs(cleanup_interval_secs);
    let bg_token = shutdown_token.clone();
    tokio::spawn(async move {
        let mut interval = tokio::time::interval(cleanup_interval);
        interval.set_missed_tick_behavior(tokio::time::MissedTickBehavior::Delay);
        interval.tick().await; // skip the immediate first tick
        loop {
            tokio::select! {
                _ = interval.tick() => {}
                _ = bg_token.cancelled() => {
                    tracing::info!("background cleanup task shutting down");
                    return;
                }
            }
            match session_service::cleanup_expired_sessions(&cleanup_pool).await {
                Ok(count) if count > 0 => {
                    tracing::info!(count, "cleaned up expired sessions");
                }
                Err(e) => {
                    tracing::debug!(error = %e, "session cleanup error details");
                    tracing::error!("failed to cleanup expired sessions");
                }
                _ => {}
            }
            match agent_session_output_service::cleanup_old_outputs(
                &cleanup_pool,
                output_retention_days,
            )
            .await
            {
                Ok(count) if count > 0 => {
                    tracing::info!(count, "cleaned up old agent outputs");
                }
                Err(e) => {
                    tracing::debug!(error = %e, "output cleanup error details");
                    tracing::error!("failed to cleanup old agent outputs");
                }
                _ => {}
            }

            // Periodic task_locks cleanup to prevent unbounded growth
            cleanup_orchestrator.cleanup_task_locks().await;
        }
    });

    // Spawn background GitHub sync polling (if enabled)
    if let Some(sync_client) = sync_github_client {
        let sync_interval_secs = config.github_sync_interval_secs.max(60);
        let sync_hub = sync_sse_hub;
        let sync_token = shutdown_token.clone();
        tokio::spawn(async move {
            let mut interval = tokio::time::interval(Duration::from_secs(sync_interval_secs));
            interval.set_missed_tick_behavior(tokio::time::MissedTickBehavior::Delay);
            interval.tick().await; // skip the immediate first tick
            loop {
                tokio::select! {
                    _ = interval.tick() => {}
                    _ = sync_token.cancelled() => {
                        tracing::info!("background GitHub sync task shutting down");
                        return;
                    }
                }
                let engine = gantry_board::github::sync_engine::SyncEngine::new(
                    Arc::clone(&sync_client),
                    sync_pool.clone(),
                );
                match engine.sync_all().await {
                    Ok(results) => {
                        for r in results {
                            if r.pushed > 0 || r.pulled > 0 {
                                tracing::info!(
                                    project_id = %r.project_id,
                                    pushed = r.pushed,
                                    pulled = r.pulled,
                                    "GitHub sync completed"
                                );
                            }
                            sync_hub.broadcast(
                                gantry_board::sse::event::SseEvent::github_sync_completed(r),
                            );
                        }
                    }
                    Err(e) => {
                        tracing::warn!(error = %e, "background GitHub sync failed");
                    }
                }
            }
        });
        tracing::info!(
            interval_secs = sync_interval_secs,
            "background GitHub sync polling started"
        );
    }

    let listener = tokio::net::TcpListener::bind(&config.bind_addr).await?;
    tracing::info!("listening on {}", config.bind_addr);

    axum::serve(
        listener,
        app.into_make_service_with_connect_info::<std::net::SocketAddr>(),
    )
    .with_graceful_shutdown(shutdown_signal())
    .await?;

    // Gracefully shut down running agent sessions (with timeout)
    if tokio::time::timeout(
        Duration::from_secs(30),
        shutdown_orchestrator.shutdown_gracefully(),
    )
    .await
    .is_err()
    {
        tracing::warn!("orchestrator graceful shutdown timed out after 30s");
    }

    // Signal background tasks to stop
    shutdown_token.cancel();
    tracing::info!("shutdown complete");

    Ok(())
}

async fn shutdown_signal() {
    let ctrl_c = tokio::signal::ctrl_c();

    #[cfg(unix)]
    {
        let mut sigterm = tokio::signal::unix::signal(tokio::signal::unix::SignalKind::terminate())
            .expect("failed to install SIGTERM handler");
        tokio::select! {
            _ = ctrl_c => tracing::info!("received SIGINT, starting graceful shutdown"),
            _ = sigterm.recv() => tracing::info!("received SIGTERM, starting graceful shutdown"),
        }
    }

    #[cfg(not(unix))]
    {
        ctrl_c.await.ok();
        tracing::info!("received SIGINT, starting graceful shutdown");
    }
}
