pub mod agent;
pub mod auth;
pub mod config;
pub mod db;
pub mod error;
pub mod github;
pub mod handlers;
pub mod models;
pub mod observability;
pub mod openapi;
pub mod realtime;
pub mod repositories;
pub mod services;
#[cfg(test)]
pub mod test_helpers;

use std::sync::atomic::AtomicUsize;
use std::sync::{Arc, OnceLock};
use std::time::Duration;

use axum::http::Method;
use axum::routing::{delete, get, patch, post};
use axum::Router;
use axum_prometheus::metrics_exporter_prometheus::PrometheusHandle;
use axum_prometheus::{GenericMetricLayer, Handle};
use sqlx::SqlitePool;
use tower_governor::governor::GovernorConfigBuilder;
use tower_governor::GovernorLayer;
use tower_http::cors::{AllowOrigin, CorsLayer};
use tower_http::request_id::{MakeRequestUuid, PropagateRequestIdLayer, SetRequestIdLayer};
use tower_http::timeout::TimeoutLayer;
use tower_http::trace::TraceLayer;
use utoipa::OpenApi;
use utoipa_swagger_ui::SwaggerUi;

use crate::agent::orchestrator::AgentOrchestrator;
use crate::github::api::GitHubApi;
use crate::realtime::hub::SseHub;
use crate::services::agent_session_output_service::OutputBuffer;
use crate::services::preview_service::PreviewManager;

#[derive(Clone)]
pub struct AppState {
    pub pool: SqlitePool,
    pub sse_hub: Arc<SseHub>,
    pub config: Arc<config::Config>,
    pub orchestrator: Arc<AgentOrchestrator>,
    pub preview_manager: Option<Arc<PreviewManager>>,
    pub github_client: Option<Arc<dyn GitHubApi>>,
    pub output_buffer: Arc<OutputBuffer>,
    /// Shared counter for active SSE + WebSocket connections (for limit enforcement).
    pub connection_counter: Arc<AtomicUsize>,
    pub started_at: std::time::Instant,
}

macro_rules! governor {
    ($per_second:expr, $burst_size:expr) => {
        GovernorConfigBuilder::default()
            .per_second($per_second)
            .burst_size($burst_size)
            .finish()
            .ok_or_else(|| {
                config::ConfigError::RateLimiter(format!(
                    "per_second={}, burst_size={}",
                    $per_second, $burst_size
                ))
            })
    };
}

pub fn app(state: AppState) -> Result<Router, config::ConfigError> {
    // Rate limit: login — configurable (default: 5 attempts per 15 min per IP).
    // per_second(N) sets the replenishment *interval* to N seconds (NOT N req/sec).
    let login_governor = governor!(
        state.config.login_rate_limit_per_second,
        state.config.login_rate_limit_burst
    )?;
    // Rate limit: register — configurable (default: 3 attempts per hour per IP).
    let register_governor = governor!(
        state.config.register_rate_limit_per_second,
        state.config.register_rate_limit_burst
    )?;
    // General API rate limit — configurable (default: ~1 req/s sustained, 60-burst per IP).
    let general_governor = governor!(
        state.config.general_rate_limit_per_second,
        state.config.general_rate_limit_burst
    )?;
    // Rate limit: agent start — 1 per 10 seconds, burst 3 per IP.
    let agent_start_governor = governor!(10, 3)?;
    // Rate limit: agent restart — same as start.
    let agent_restart_governor = governor!(10, 3)?;
    // Rate limit: SSE/WebSocket connections — 5 connections per 10 seconds per IP.
    let sse_governor = governor!(10, 5)?;
    let ws_governor = governor!(10, 5)?;

    let api_routes = Router::new()
        // Auth endpoints (rate-limited)
        .route(
            "/auth/register",
            post(handlers::auth::register).layer(GovernorLayer::new(register_governor)),
        )
        .route(
            "/auth/login",
            post(handlers::auth::login).layer(GovernorLayer::new(login_governor)),
        )
        .route("/auth/logout", post(handlers::auth::logout))
        .route("/auth/me", get(handlers::auth::me))
        // Task endpoints
        .route("/tasks", get(handlers::tasks::list_tasks))
        .route("/tasks", post(handlers::tasks::create_task))
        .route("/tasks/{id}", get(handlers::tasks::get_task))
        .route("/tasks/{id}", patch(handlers::tasks::update_task))
        .route("/tasks/{id}", delete(handlers::tasks::delete_task))
        // Project endpoints
        .route("/projects", get(handlers::projects::list_projects))
        .route("/projects", post(handlers::projects::create_project))
        .route("/projects/{id}", get(handlers::projects::get_project))
        .route("/projects/{id}", patch(handlers::projects::update_project))
        .route("/projects/{id}", delete(handlers::projects::delete_project))
        // User endpoints
        .route("/users", get(handlers::users::search_users))
        // Project members
        .route(
            "/projects/{project_id}/members",
            get(handlers::project_members::list_members),
        )
        .route(
            "/projects/{project_id}/members",
            post(handlers::project_members::add_member),
        )
        .route(
            "/projects/{project_id}/members/{user_id}",
            get(handlers::project_members::get_member),
        )
        .route(
            "/projects/{project_id}/members/{user_id}",
            patch(handlers::project_members::update_member),
        )
        .route(
            "/projects/{project_id}/members/{user_id}",
            delete(handlers::project_members::remove_member),
        )
        // Project message endpoints
        .route(
            "/projects/{project_id}/messages",
            get(handlers::project_messages::list_messages),
        )
        .route(
            "/projects/{project_id}/messages",
            post(handlers::project_messages::create_message),
        )
        .route(
            "/projects/{project_id}/messages/{message_id}",
            delete(handlers::project_messages::delete_message),
        )
        // Project invitation endpoints
        .route(
            "/projects/{project_id}/invitations",
            get(handlers::invitations::list_invitations),
        )
        .route(
            "/projects/{project_id}/invitations",
            post(handlers::invitations::create_invitation),
        )
        .route(
            "/projects/{project_id}/invitations/{invitation_id}",
            delete(handlers::invitations::delete_invitation),
        )
        .route(
            "/invitations/{token}",
            get(handlers::invitations::get_invitation_by_token),
        )
        .route(
            "/invitations/{token}/accept",
            post(handlers::invitations::accept_invitation),
        )
        // Task comment endpoints
        .route(
            "/tasks/{task_id}/comments",
            get(handlers::task_comments::list_comments),
        )
        .route(
            "/tasks/{task_id}/comments",
            post(handlers::task_comments::create_comment),
        )
        .route(
            "/tasks/{task_id}/comments/{comment_id}",
            patch(handlers::task_comments::update_comment),
        )
        .route(
            "/tasks/{task_id}/comments/{comment_id}",
            delete(handlers::task_comments::delete_comment),
        )
        // Pull request endpoints
        .route(
            "/tasks/{task_id}/pull-requests",
            get(handlers::pull_requests::list_pull_requests),
        )
        // Agent session endpoints
        .route(
            "/tasks/{task_id}/sessions",
            get(handlers::agent_sessions::list_agent_sessions),
        )
        .route(
            "/tasks/{task_id}/sessions",
            post(handlers::agent_sessions::create_agent_session),
        )
        .route(
            "/tasks/{task_id}/sessions/{session_id}",
            get(handlers::agent_sessions::get_agent_session),
        )
        .route(
            "/tasks/{task_id}/sessions/{session_id}",
            patch(handlers::agent_sessions::update_agent_session),
        )
        .route(
            "/tasks/{task_id}/sessions/start",
            post(handlers::agent_sessions::start_agent_session)
                .layer(GovernorLayer::new(agent_start_governor)),
        )
        .route(
            "/tasks/{task_id}/sessions/{session_id}/stop",
            post(handlers::agent_sessions::stop_agent_session),
        )
        .route(
            "/tasks/{task_id}/sessions/{session_id}/pause",
            post(handlers::agent_sessions::pause_agent_session),
        )
        .route(
            "/tasks/{task_id}/sessions/{session_id}/resume",
            post(handlers::agent_sessions::resume_agent_session),
        )
        .route(
            "/tasks/{task_id}/sessions/{session_id}/outputs",
            get(handlers::agent_sessions::get_agent_session_outputs),
        )
        .route(
            "/tasks/{task_id}/sessions/{session_id}/restart",
            post(handlers::agent_sessions::restart_agent_session)
                .layer(GovernorLayer::new(agent_restart_governor)),
        )
        // Worktree endpoints (global, legacy)
        .route("/worktrees", get(handlers::worktrees::list_worktrees))
        .route("/worktrees", post(handlers::worktrees::create_worktree))
        .route("/worktrees/{name}", get(handlers::worktrees::get_worktree))
        .route(
            "/worktrees/{name}",
            delete(handlers::worktrees::delete_worktree),
        )
        // Worktree endpoints (project-scoped)
        .route(
            "/projects/{project_id}/worktrees",
            get(handlers::worktrees::list_project_worktrees),
        )
        .route(
            "/projects/{project_id}/worktrees",
            post(handlers::worktrees::create_project_worktree),
        )
        .route(
            "/projects/{project_id}/worktrees/{name}",
            get(handlers::worktrees::get_project_worktree),
        )
        .route(
            "/projects/{project_id}/worktrees/{name}",
            delete(handlers::worktrees::delete_project_worktree),
        )
        // GitHub link endpoints
        .route(
            "/projects/{project_id}/github-link",
            post(handlers::github_links::create_github_link),
        )
        .route(
            "/projects/{project_id}/github-link",
            get(handlers::github_links::get_github_link),
        )
        .route(
            "/projects/{project_id}/github-link",
            delete(handlers::github_links::delete_github_link),
        )
        .route(
            "/projects/{project_id}/github-link/status",
            get(handlers::github_links::get_github_link_status),
        )
        .route(
            "/projects/{project_id}/github-link/sync",
            post(handlers::github_links::sync_github_link),
        )
        // Preview endpoints
        .route("/previews", get(handlers::previews::list_previews))
        .route("/previews", post(handlers::previews::create_preview))
        .route("/previews/{id}", get(handlers::previews::get_preview))
        .route("/previews/{id}", delete(handlers::previews::delete_preview))
        .route(
            "/previews/{id}/start",
            post(handlers::previews::start_preview),
        )
        .route(
            "/previews/{id}/stop",
            post(handlers::previews::stop_preview),
        )
        .route(
            "/previews/{id}/restart",
            post(handlers::previews::restart_preview),
        )
        // Admin endpoints
        .route("/admin/status", get(handlers::admin::admin_status))
        .route("/admin/audit-log", get(handlers::admin::audit_log))
        // CSRF protection: require X-Requested-With header on state-changing requests
        .layer(axum::middleware::from_fn(auth::csrf::csrf_check))
        // General API rate limit applied to all routes
        .layer(GovernorLayer::new(general_governor))
        // Request timeout (SSE excluded — it's merged after this layer)
        .layer(TimeoutLayer::with_status_code(
            axum::http::StatusCode::REQUEST_TIMEOUT,
            Duration::from_secs(state.config.request_timeout_secs),
        ))
        // SSE route: no timeout (long-lived streaming), own rate limit only
        .merge(Router::new().route(
            "/events",
            get(realtime::handler::sse_handler).layer(GovernorLayer::new(sse_governor)),
        ))
        // WebSocket route: no timeout (long-lived), own rate limit only
        .merge(Router::new().route(
            "/ws",
            get(realtime::ws_handler::ws_handler).layer(GovernorLayer::new(ws_governor)),
        ));

    let metric_handle = init_metrics();
    let (prometheus_layer, _) = GenericMetricLayer::<'_, PrometheusHandle, Handle>::pair_from(
        Handle(metric_handle.clone()),
    );
    // Seed custom business metrics after GenericMetricLayer is created
    // (must happen after recorder setup is fully complete)
    observability::init_metrics();

    let router =
        Router::new()
            .route("/health", get(handlers::health::health_check))
            .route("/health/live", get(handlers::health::liveness))
            .route("/health/ready", get(handlers::health::readiness))
            .route(
                "/metrics",
                get(move |_user: crate::auth::middleware::AuthUser| async move {
                    metric_handle.render()
                }),
            )
            .nest("/api", api_routes)
            // Webhook route: no auth/CSRF, body limit 25MB
            .route(
                "/api/webhooks/github",
                post(handlers::webhooks::github_webhook)
                    .layer(axum::extract::DefaultBodyLimit::max(25 * 1024 * 1024)),
            )
            // Default body size limit: 2 MB (webhook keeps its own 25 MB limit above)
            .layer(axum::extract::DefaultBodyLimit::max(2 * 1024 * 1024));

    // Swagger UI is only available in debug builds (development)
    #[cfg(debug_assertions)]
    let router = router.merge(
        SwaggerUi::new("/swagger-ui").url("/api-docs/openapi.json", openapi::ApiDoc::openapi()),
    );

    Ok(router
        .layer(axum::middleware::from_fn(
            error::inject_request_id_into_errors,
        ))
        .layer(prometheus_layer)
        .layer(build_cors_layer(&state.config)?)
        .layer(axum::middleware::from_fn_with_state(
            state.clone(),
            auth::host_check::validate_host_header,
        ))
        .layer(PropagateRequestIdLayer::x_request_id())
        .layer(axum::middleware::from_fn(inject_request_path))
        .layer(
            TraceLayer::new_for_http()
                .make_span_with(observability::AccessLogMakeSpan)
                .on_response(observability::AccessLogOnResponse),
        )
        .layer(SetRequestIdLayer::x_request_id(MakeRequestUuid))
        .with_state(state))
}

/// Initialize the Prometheus metrics recorder once (safe for repeated calls in tests).
fn init_metrics() -> PrometheusHandle {
    static HANDLE: OnceLock<PrometheusHandle> = OnceLock::new();
    HANDLE
        .get_or_init(|| {
            let recorder =
                axum_prometheus::metrics_exporter_prometheus::PrometheusBuilder::default()
                    .build_recorder();
            let handle = recorder.handle();
            // Ignore error if another recorder was already installed
            let _ = metrics::set_global_recorder(recorder);
            handle
        })
        .clone()
}

/// Middleware that stores the request path in response extensions
/// so that `AccessLogOnResponse` can determine log level based on path.
async fn inject_request_path(
    request: axum::extract::Request,
    next: axum::middleware::Next,
) -> axum::response::Response {
    let path = request.uri().path().to_string();
    let mut response = next.run(request).await;
    response
        .extensions_mut()
        .insert(observability::RequestPath(path));
    response
}

fn build_cors_layer(config: &config::Config) -> Result<CorsLayer, config::ConfigError> {
    match config.cors_origin_header()? {
        Some(origin) => Ok(CorsLayer::new()
            .allow_origin(AllowOrigin::exact(origin))
            .allow_methods([Method::GET, Method::POST, Method::PATCH, Method::DELETE])
            .allow_headers([
                axum::http::header::CONTENT_TYPE,
                axum::http::HeaderName::from_static("x-requested-with"),
            ])
            .allow_credentials(true)),
        None => {
            // Defense in depth: release builds must never reach this branch.
            // Config::validate() already returns Err if cors_origin is None in release,
            // but we guard here too in case validate() is bypassed.
            #[cfg(not(debug_assertions))]
            return Err(config::ConfigError::MissingCorsOriginInRelease);

            #[cfg(debug_assertions)]
            {
                tracing::warn!(
                    "GANTRY_CORS_ORIGIN is not set — CORS is permissive (debug build only)"
                );
                Ok(CorsLayer::permissive())
            }
        }
    }
}
