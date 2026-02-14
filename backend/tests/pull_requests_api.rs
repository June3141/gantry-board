mod common;

use axum::http::StatusCode;
use common::{create_test_server_with_pool, SqlitePool};

async fn create_test_project(server: &axum_test::TestServer) -> String {
    let response = server
        .post("/api/projects")
        .json(&serde_json::json!({
            "name": "Test Project",
            "description": "A test project"
        }))
        .await;
    response.assert_status(StatusCode::CREATED);
    let body: serde_json::Value = response.json();
    body["id"].as_str().unwrap().to_string()
}

async fn create_test_task(server: &axum_test::TestServer, project_id: &str) -> String {
    let response = server
        .post("/api/tasks")
        .json(&serde_json::json!({
            "project_id": project_id,
            "title": "Test Task"
        }))
        .await;
    response.assert_status(StatusCode::CREATED);
    let body: serde_json::Value = response.json();
    body["id"].as_str().unwrap().to_string()
}

async fn insert_pr(pool: &SqlitePool, github_link_id: &str, task_id: &str, pr_number: i64) {
    let id = uuid::Uuid::new_v4().to_string();
    sqlx::query(
        r#"INSERT INTO github_pull_requests
        (id, github_link_id, task_id, pr_number, title, url, state, is_merged, author)
        VALUES ($1, $2, $3, $4, $5, $6, $7, $8, $9)"#,
    )
    .bind(&id)
    .bind(github_link_id)
    .bind(task_id)
    .bind(pr_number)
    .bind(format!("PR #{pr_number}"))
    .bind(format!("https://github.com/owner/repo/pull/{pr_number}"))
    .bind("open")
    .bind(false)
    .bind("octocat")
    .execute(pool)
    .await
    .unwrap();
}

async fn insert_github_link(pool: &SqlitePool, project_id: &str) -> String {
    let id = uuid::Uuid::new_v4().to_string();
    sqlx::query(
        "INSERT INTO github_links (id, project_id, repo_owner, repo_name) VALUES ($1, $2, $3, $4)",
    )
    .bind(&id)
    .bind(project_id)
    .bind("owner")
    .bind("repo")
    .execute(pool)
    .await
    .unwrap();
    id
}

#[tokio::test]
async fn test_list_prs_empty() {
    let (server, _pool) = create_test_server_with_pool().await;
    let project_id = create_test_project(&server).await;
    let task_id = create_test_task(&server, &project_id).await;

    let response = server
        .get(&format!("/api/tasks/{task_id}/pull-requests"))
        .await;

    response.assert_status_ok();
    let prs: Vec<serde_json::Value> = response.json();
    assert!(prs.is_empty());
}

#[tokio::test]
async fn test_list_prs_after_insert() {
    let (server, pool) = create_test_server_with_pool().await;
    let project_id = create_test_project(&server).await;
    let task_id = create_test_task(&server, &project_id).await;
    let link_id = insert_github_link(&pool, &project_id).await;

    insert_pr(&pool, &link_id, &task_id, 42).await;
    insert_pr(&pool, &link_id, &task_id, 43).await;

    let response = server
        .get(&format!("/api/tasks/{task_id}/pull-requests"))
        .await;

    response.assert_status_ok();
    let prs: Vec<serde_json::Value> = response.json();
    assert_eq!(prs.len(), 2);
    assert_eq!(prs[0]["pr_number"], 42);
    assert_eq!(prs[0]["title"], "PR #42");
    assert_eq!(prs[0]["author"], "octocat");
    assert_eq!(prs[1]["pr_number"], 43);
}

#[tokio::test]
async fn test_list_prs_not_found_returns_404() {
    let (server, _pool) = create_test_server_with_pool().await;
    let fake_task_id = uuid::Uuid::new_v4();

    let response = server
        .get(&format!("/api/tasks/{fake_task_id}/pull-requests"))
        .await;

    response.assert_status(StatusCode::NOT_FOUND);
}
