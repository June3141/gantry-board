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

async fn create_test_server_impl(auth_disabled: bool, repo_path: PathBuf) -> TestServer {
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
        pool,
        sse_hub,
        config: Arc::new(config),
        orchestrator,
    };

    let app = gantry_board::app(state).into_make_service_with_connect_info::<SocketAddr>();
    TestServer::new(app).expect("Failed to create test server")
}

/// Create a test server with auth disabled (for CRUD tests).
pub async fn create_test_server() -> TestServer {
    create_test_server_impl(true, PathBuf::from(".")).await
}

/// Create a test server with auth enabled (for auth/authorization tests).
pub async fn create_auth_test_server() -> TestServer {
    create_test_server_impl(false, PathBuf::from(".")).await
}

/// Create a test server with a temporary git repo (for agent session tests).
pub async fn create_test_server_with_repo() -> (tempfile::TempDir, TestServer) {
    let tmp = tempfile::TempDir::new().expect("Failed to create temp dir");
    let repo_path = tmp.path().join("repo");
    std::fs::create_dir(&repo_path).expect("Failed to create repo dir");
    let repo = git2::Repository::init(&repo_path).expect("Failed to init repo");
    let sig = git2::Signature::now("test", "test@test.com").unwrap();
    let tree_id = repo.index().unwrap().write_tree().unwrap();
    let tree = repo.find_tree(tree_id).unwrap();
    repo.commit(Some("HEAD"), &sig, &sig, "initial", &tree, &[])
        .unwrap();

    let server = create_test_server_impl(true, repo_path).await;
    (tmp, server)
}
