pub mod agent;
pub mod auth;
pub mod config;
pub mod db;
pub mod error;
pub mod handlers;
pub mod models;
pub mod openapi;
pub mod services;
pub mod sse;
#[cfg(test)]
pub mod test_helpers;

use std::sync::Arc;

use axum::http::Method;
use axum::routing::{delete, get, patch, post};
use axum::Router;
use sqlx::SqlitePool;
use tower_governor::governor::GovernorConfigBuilder;
use tower_governor::GovernorLayer;
use tower_http::cors::{AllowOrigin, CorsLayer};
use tower_http::trace::TraceLayer;
use utoipa::OpenApi;
use utoipa_swagger_ui::SwaggerUi;

use crate::agent::orchestrator::AgentOrchestrator;
use crate::sse::hub::SseHub;

#[derive(Clone)]
pub struct AppState {
    pub pool: SqlitePool,
    pub sse_hub: Arc<SseHub>,
    pub config: Arc<config::Config>,
    pub orchestrator: Arc<AgentOrchestrator>,
}

pub fn app(state: AppState) -> Router {
    // Rate limit: login — 5 attempts per 15 min per IP.
    // per_second(N) sets the replenishment *interval* to N seconds (NOT N req/sec).
    let login_governor = GovernorConfigBuilder::default()
        .per_second(180) // 1 token every 180s
        .burst_size(5) // bucket capacity
        .finish()
        .expect("valid governor config");

    // Rate limit: register — 3 attempts per hour per IP.
    let register_governor = GovernorConfigBuilder::default()
        .per_second(1200) // 1 token every 1200s
        .burst_size(3) // bucket capacity
        .finish()
        .expect("valid governor config");

    // General API rate limit: ~1 req/s sustained, 60-request burst capacity per IP.
    let general_governor = GovernorConfigBuilder::default()
        .per_second(1) // refill 1 token per second
        .burst_size(60) // bucket capacity: allows initial burst up to 60
        .finish()
        .expect("valid governor config");

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
            post(handlers::agent_sessions::start_agent_session),
        )
        .route(
            "/tasks/{task_id}/sessions/{session_id}/stop",
            post(handlers::agent_sessions::stop_agent_session),
        )
        .route(
            "/tasks/{task_id}/sessions/{session_id}/outputs",
            get(handlers::agent_sessions::get_agent_session_outputs),
        )
        // SSE for real-time updates
        .route("/events", get(sse::handler::sse_handler))
        // General API rate limit applied to all routes
        .layer(GovernorLayer::new(general_governor));

    Router::new()
        .route("/health", get(handlers::health::health_check))
        .nest("/api", api_routes)
        .merge(
            SwaggerUi::new("/swagger-ui").url("/api-docs/openapi.json", openapi::ApiDoc::openapi()),
        )
        .layer(build_cors_layer(&state.config))
        .layer(TraceLayer::new_for_http())
        .with_state(state)
}

fn build_cors_layer(config: &config::Config) -> CorsLayer {
    match config.cors_origin_header() {
        Some(origin) => CorsLayer::new()
            .allow_origin(AllowOrigin::exact(origin))
            .allow_methods([Method::GET, Method::POST, Method::PATCH, Method::DELETE])
            .allow_headers([axum::http::header::CONTENT_TYPE])
            .allow_credentials(true),
        None => {
            // Defense in depth: release builds must never reach this branch.
            // Config::validate() already panics if cors_origin is None in release,
            // but we guard here too in case validate() is bypassed.
            #[cfg(not(debug_assertions))]
            panic!("GANTRY_CORS_ORIGIN must be set in production");

            #[cfg(debug_assertions)]
            {
                tracing::warn!(
                    "GANTRY_CORS_ORIGIN is not set — CORS is permissive (debug build only)"
                );
                CorsLayer::permissive()
            }
        }
    }
}
