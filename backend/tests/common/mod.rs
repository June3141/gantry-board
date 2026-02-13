#![allow(dead_code)]

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
pub use sqlx::SqlitePool;

async fn create_test_server_impl(
    auth_disabled: bool,
    repo_path: PathBuf,
) -> (TestServer, SqlitePool) {
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
        auth_disabled,
        repository_path: Some(repo_path.to_string_lossy().to_string()),
        ..Default::default()
    };

    let sse_hub = Arc::new(SseHub::default());
    let mut executors: HashMap<AgentType, Arc<dyn AgentExecutor>> = HashMap::new();
    executors.insert(AgentType::ClaudeCode, Arc::new(NoopExecutor));
    let orchestrator = Arc::new(AgentOrchestrator::new(
        executors,
        pool.clone(),
        repo_path,
        Arc::clone(&sse_hub),
    ));
    let state = AppState {
        pool: pool.clone(),
        sse_hub,
        config: Arc::new(config),
        orchestrator,
        preview_manager: None,
        started_at: std::time::Instant::now(),
    };

    let app = gantry_board::app(state)
        .expect("Failed to build app")
        .into_make_service_with_connect_info::<SocketAddr>();
    let mut server = TestServer::new(app).expect("Failed to create test server");
    // CSRF middleware requires X-Requested-With on state-changing requests
    server.add_header("x-requested-with", "XMLHttpRequest");
    (server, pool)
}

/// Create a test server with auth disabled (for CRUD tests).
pub async fn create_test_server() -> TestServer {
    create_test_server_impl(true, PathBuf::from(".")).await.0
}

/// Create a test server with auth enabled (for auth/authorization tests).
pub async fn create_auth_test_server() -> TestServer {
    create_test_server_impl(false, PathBuf::from(".")).await.0
}

fn init_test_repo() -> (tempfile::TempDir, PathBuf) {
    let tmp = tempfile::TempDir::new().expect("Failed to create temp dir");
    let repo_path = tmp.path().join("repo");
    std::fs::create_dir(&repo_path).expect("Failed to create repo dir");
    let repo = git2::Repository::init(&repo_path).expect("Failed to init repo");
    let sig = git2::Signature::now("test", "test@test.com").unwrap();
    let tree_id = repo.index().unwrap().write_tree().unwrap();
    let tree = repo.find_tree(tree_id).unwrap();
    repo.commit(Some("HEAD"), &sig, &sig, "initial", &tree, &[])
        .unwrap();
    (tmp, repo_path)
}

/// Create a test server with auth disabled and DB pool access.
pub async fn create_test_server_with_pool() -> (TestServer, SqlitePool) {
    create_test_server_impl(true, PathBuf::from(".")).await
}

/// Create a test server with a temporary git repo (for agent session tests).
pub async fn create_test_server_with_repo() -> (tempfile::TempDir, TestServer) {
    let (tmp, repo_path) = init_test_repo();
    let (server, _pool) = create_test_server_impl(true, repo_path).await;
    (tmp, server)
}

/// Create a test server with a temporary git repo and auth enabled.
pub async fn create_auth_test_server_with_repo() -> (tempfile::TempDir, TestServer) {
    let (tmp, repo_path) = init_test_repo();
    let (server, _pool) = create_test_server_impl(false, repo_path).await;
    (tmp, server)
}

/// Create a test server with a temporary git repo and DB pool access.
pub async fn create_test_server_with_repo_and_pool() -> (tempfile::TempDir, TestServer, SqlitePool)
{
    let (tmp, repo_path) = init_test_repo();
    let (server, pool) = create_test_server_impl(true, repo_path).await;
    (tmp, server, pool)
}
