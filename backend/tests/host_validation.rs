mod common;

use axum::http::StatusCode;
use std::collections::HashMap;
use std::net::SocketAddr;
use std::path::PathBuf;
use std::sync::Arc;

use axum_test::TestServer;
use gantry_board::agent::executor::{AgentExecutor, NoopExecutor};
use gantry_board::agent::orchestrator::AgentOrchestrator;
use gantry_board::config::Config;
use gantry_board::models::agent_session::AgentType;
use gantry_board::sse::hub::SseHub;
use gantry_board::AppState;
use sqlx::sqlite::SqlitePoolOptions;

async fn create_server_with_allowed_hosts(hosts: Vec<String>) -> TestServer {
    let pool = SqlitePoolOptions::new()
        .connect("sqlite::memory:")
        .await
        .expect("Failed to create test database");

    sqlx::migrate!("./migrations")
        .run(&pool)
        .await
        .expect("Failed to run migrations");

    let config = Config {
        bind_addr: "127.0.0.1:0".to_string(),
        database_url: "sqlite::memory:".to_string(),
        auth_disabled: true,
        allowed_hosts: hosts,
        ..Default::default()
    };

    let sse_hub = Arc::new(SseHub::default());
    let mut executors: HashMap<AgentType, Arc<dyn AgentExecutor>> = HashMap::new();
    executors.insert(AgentType::ClaudeCode, Arc::new(NoopExecutor));
    let orchestrator = Arc::new(AgentOrchestrator::new(
        executors,
        pool.clone(),
        PathBuf::from("."),
        Arc::clone(&sse_hub),
    ));
    let state = AppState {
        pool,
        sse_hub,
        config: Arc::new(config),
        orchestrator,
        preview_manager: None,
        github_client: None,
        started_at: std::time::Instant::now(),
    };

    let app = gantry_board::app(state)
        .expect("Failed to build app")
        .into_make_service_with_connect_info::<SocketAddr>();
    let mut server = TestServer::new(app).expect("Failed to create test server");
    server.add_header("x-requested-with", "XMLHttpRequest");
    server
}

#[tokio::test]
async fn test_host_validation_skipped_when_no_hosts_configured() {
    let server = create_server_with_allowed_hosts(vec![]).await;

    let response = server.get("/health").await;
    response.assert_status_ok();
}

#[tokio::test]
async fn test_host_validation_allows_configured_host() {
    // axum_test sends to 127.0.0.1:<port>, host header defaults to that
    let server = create_server_with_allowed_hosts(vec!["localhost:3000".to_string()]).await;

    // The test server uses its own address as host, so we add the correct host header
    let response = server
        .get("/health")
        .add_header("host", "localhost:3000")
        .await;
    response.assert_status_ok();
}

#[tokio::test]
async fn test_host_validation_rejects_unknown_host() {
    let server = create_server_with_allowed_hosts(vec!["allowed.example.com".to_string()]).await;

    let response = server
        .get("/health")
        .add_header("host", "evil.example.com")
        .await;
    response.assert_status(StatusCode::BAD_REQUEST);
}
