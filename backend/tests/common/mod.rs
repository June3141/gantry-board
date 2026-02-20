#![allow(dead_code)]

use std::collections::HashMap;
use std::net::SocketAddr;
use std::path::PathBuf;
use std::sync::Arc;

use axum::http::header;
use axum_test::TestServer;
use gantry_board::agent::executor::{
    AgentConfig, AgentExecutor, AgentHandle, AgentOutputEvent, NoopExecutor,
};
use gantry_board::agent::orchestrator::AgentOrchestrator;
use gantry_board::config::Config;
use gantry_board::error::AppResult;
use gantry_board::models::agent_session::AgentType;
use gantry_board::services::agent_session_output_service::OutputBuffer;
use gantry_board::sse::hub::SseHub;
use gantry_board::AppState;
use sqlx::sqlite::SqlitePoolOptions;
pub use sqlx::SqlitePool;
use tokio::sync::mpsc;
use tokio_util::sync::CancellationToken;

/// Executor that spawns a real `sleep` process for testing pause/resume with signals.
pub struct SleepExecutor;

#[async_trait::async_trait]
impl AgentExecutor for SleepExecutor {
    async fn start(&self, _config: AgentConfig) -> AppResult<AgentHandle> {
        use std::process::Stdio;
        use tokio::process::Command;

        let mut child = Command::new("sleep")
            .arg("3600")
            .stdin(Stdio::null())
            .stdout(Stdio::piped())
            .stderr(Stdio::null())
            .kill_on_drop(true)
            .spawn()
            .expect("failed to spawn sleep process");

        let pid = child.id();
        let cancel = CancellationToken::new();
        let (tx, rx) = mpsc::channel(16);
        let token = cancel.clone();

        let join_handle = tokio::spawn(async move {
            token.cancelled().await;
            let _ = child.kill().await;
            let _ = child.wait().await;
            let _ = tx.send(AgentOutputEvent::Completed).await;
            Ok(())
        });

        Ok(AgentHandle {
            cancel,
            output_rx: rx,
            join_handle,
            pid,
        })
    }
}

async fn create_test_server_with_executor(
    auth_disabled: bool,
    repo_path: PathBuf,
    executor: Arc<dyn AgentExecutor>,
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
    executors.insert(AgentType::ClaudeCode, executor);
    let output_buffer = Arc::new(OutputBuffer::new(pool.clone()));
    let orchestrator = Arc::new(AgentOrchestrator::new(
        executors,
        pool.clone(),
        repo_path,
        Arc::clone(&sse_hub),
        Arc::clone(&output_buffer),
    ));
    let state = AppState {
        pool: pool.clone(),
        sse_hub,
        config: Arc::new(config),
        orchestrator,
        preview_manager: None,
        github_client: None,
        output_buffer,
        started_at: std::time::Instant::now(),
    };

    let app = gantry_board::app(state)
        .expect("Failed to build app")
        .into_make_service_with_connect_info::<SocketAddr>();
    let mut server = TestServer::new(app).expect("Failed to create test server");
    server.add_header("x-requested-with", "XMLHttpRequest");
    (server, pool)
}

async fn create_test_server_impl(
    auth_disabled: bool,
    repo_path: PathBuf,
) -> (TestServer, SqlitePool) {
    create_test_server_with_executor(auth_disabled, repo_path, Arc::new(NoopExecutor)).await
}

/// Create a test server with auth disabled (for CRUD tests).
pub async fn create_test_server() -> TestServer {
    create_test_server_impl(true, PathBuf::from(".")).await.0
}

/// Create a test server with auth enabled (for auth/authorization tests).
pub async fn create_auth_test_server() -> TestServer {
    create_test_server_impl(false, PathBuf::from(".")).await.0
}

/// Create a test server with auth enabled and DB pool access.
pub async fn create_auth_test_server_with_pool() -> (TestServer, SqlitePool) {
    create_test_server_impl(false, PathBuf::from(".")).await
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

/// Create a test server with SleepExecutor for pause/resume tests.
pub async fn create_test_server_with_sleep_executor() -> (tempfile::TempDir, TestServer) {
    let (tmp, repo_path) = init_test_repo();
    let (server, _pool) =
        create_test_server_with_executor(true, repo_path, Arc::new(SleepExecutor)).await;
    (tmp, server)
}

// ========== Common Test Helpers (no-auth) ==========

/// Create a project (no auth) and return its ID.
pub async fn create_project_no_auth(server: &TestServer, name: &str) -> String {
    let response = server
        .post("/api/projects")
        .json(&serde_json::json!({ "name": name }))
        .await;
    response.assert_status(axum::http::StatusCode::CREATED);
    response.json::<serde_json::Value>()["id"]
        .as_str()
        .unwrap()
        .to_string()
}

/// Create a task in a project (no auth) and return its ID.
pub async fn create_task_no_auth(server: &TestServer, project_id: &str, title: &str) -> String {
    let response = server
        .post("/api/tasks")
        .json(&serde_json::json!({
            "project_id": project_id,
            "title": title
        }))
        .await;
    response.assert_status(axum::http::StatusCode::CREATED);
    response.json::<serde_json::Value>()["id"]
        .as_str()
        .unwrap()
        .to_string()
}

/// Create a project and a task in it, returning (project_id, task_id).
pub async fn create_project_and_task(server: &TestServer) -> (String, String) {
    let project_id = create_project_no_auth(server, "Test Project").await;
    let task_id = create_task_no_auth(server, &project_id, "Test Task").await;
    (project_id, task_id)
}

// ========== Auth Test Helpers ==========

/// Register a user and return (user_id, session_cookie_header_value)
pub async fn register_user(server: &TestServer, email: &str, name: &str) -> (String, String) {
    let response = server
        .post("/api/auth/register")
        .json(&serde_json::json!({
            "email": email,
            "name": name,
            "password": "Tr0ub4dor&3-correct-horse"
        }))
        .await;
    response.assert_status(axum::http::StatusCode::CREATED);

    let body: serde_json::Value = response.json();
    let user_id = body["user"]["id"].as_str().unwrap().to_string();

    let cookies = response
        .headers()
        .get("set-cookie")
        .unwrap()
        .to_str()
        .unwrap();
    let cookie_value = cookies.split(';').next().unwrap().to_string();

    (user_id, cookie_value)
}

/// Create a project and return its ID
pub async fn create_project(server: &TestServer, cookie: &str, name: &str) -> String {
    let response = server
        .post("/api/projects")
        .add_header(header::COOKIE, cookie)
        .json(&serde_json::json!({ "name": name }))
        .await;
    response.assert_status(axum::http::StatusCode::CREATED);
    let body: serde_json::Value = response.json();
    body["id"].as_str().unwrap().to_string()
}

/// Add a member to a project with the given role
pub async fn add_member(
    server: &TestServer,
    cookie: &str,
    project_id: &str,
    user_id: &str,
    role: &str,
) {
    let response = server
        .post(&format!("/api/projects/{}/members", project_id))
        .add_header(header::COOKIE, cookie)
        .json(&serde_json::json!({ "user_id": user_id, "role": role }))
        .await;
    response.assert_status(axum::http::StatusCode::CREATED);
}

/// Create a task in a project and return its ID
pub async fn create_task_in_project(
    server: &TestServer,
    cookie: &str,
    project_id: &str,
    title: &str,
) -> String {
    let response = server
        .post("/api/tasks")
        .add_header(header::COOKIE, cookie)
        .json(&serde_json::json!({
            "project_id": project_id,
            "title": title
        }))
        .await;
    response.assert_status(axum::http::StatusCode::CREATED);
    let body: serde_json::Value = response.json();
    body["id"].as_str().unwrap().to_string()
}
