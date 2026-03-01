use chrono::{DateTime, Utc};
use sqlx::prelude::FromRow;
use sqlx::SqlitePool;
use uuid::Uuid;

use crate::error::{AppError, AppResult};
use crate::models::project::Project;

#[derive(FromRow)]
struct ProjectRow {
    id: String,
    name: String,
    description: Option<String>,
    #[sqlx(default)]
    repository_path: Option<String>,
    created_at: DateTime<Utc>,
    updated_at: DateTime<Utc>,
}

impl TryFrom<ProjectRow> for Project {
    type Error = uuid::Error;

    fn try_from(row: ProjectRow) -> Result<Self, Self::Error> {
        Ok(Project {
            id: row.id.parse()?,
            name: row.name,
            description: row.description,
            repository_path: row.repository_path,
            created_at: row.created_at,
            updated_at: row.updated_at,
        })
    }
}

fn row_to_project(row: ProjectRow) -> AppResult<Project> {
    row.try_into()
        .map_err(|e: uuid::Error| AppError::Internal(e.to_string()))
}

fn rows_to_projects(rows: Vec<ProjectRow>) -> AppResult<Vec<Project>> {
    rows.into_iter().map(row_to_project).collect()
}

pub async fn insert(
    pool: &SqlitePool,
    id: Uuid,
    name: &str,
    description: Option<&str>,
    repository_path: Option<&str>,
    now: DateTime<Utc>,
) -> AppResult<()> {
    sqlx::query(
        r#"
        INSERT INTO projects (id, name, description, repository_path, created_at, updated_at)
        VALUES ($1, $2, $3, $4, $5, $6)
        "#,
    )
    .bind(id.to_string())
    .bind(name)
    .bind(description)
    .bind(repository_path)
    .bind(now)
    .bind(now)
    .execute(pool)
    .await?;

    Ok(())
}

pub async fn find_by_id(pool: &SqlitePool, id: Uuid) -> AppResult<Option<Project>> {
    let row = sqlx::query_as::<_, ProjectRow>(
        r#"
        SELECT id, name, description, repository_path, created_at, updated_at
        FROM projects
        WHERE id = $1
        "#,
    )
    .bind(id.to_string())
    .fetch_optional(pool)
    .await?;

    row.map(row_to_project).transpose()
}

pub async fn find_all(pool: &SqlitePool) -> AppResult<Vec<Project>> {
    let rows = sqlx::query_as::<_, ProjectRow>(
        r#"
        SELECT id, name, description, repository_path, created_at, updated_at
        FROM projects
        ORDER BY created_at DESC
        "#,
    )
    .fetch_all(pool)
    .await?;

    rows_to_projects(rows)
}

pub async fn find_all_paginated(
    pool: &SqlitePool,
    limit: i64,
    offset: i64,
) -> AppResult<(Vec<Project>, i64)> {
    let total: (i64,) = sqlx::query_as(
        r#"
        SELECT COUNT(*) FROM projects
        "#,
    )
    .fetch_one(pool)
    .await?;

    let rows = sqlx::query_as::<_, ProjectRow>(
        r#"
        SELECT id, name, description, repository_path, created_at, updated_at
        FROM projects
        ORDER BY created_at DESC
        LIMIT $1 OFFSET $2
        "#,
    )
    .bind(limit)
    .bind(offset)
    .fetch_all(pool)
    .await?;

    let projects = rows_to_projects(rows)?;
    Ok((projects, total.0))
}

pub async fn find_all_for_user(pool: &SqlitePool, user_id: Uuid) -> AppResult<Vec<Project>> {
    let rows = sqlx::query_as::<_, ProjectRow>(
        r#"
        SELECT p.id, p.name, p.description, p.repository_path, p.created_at, p.updated_at
        FROM projects p
        INNER JOIN project_members pm ON p.id = pm.project_id
        WHERE pm.user_id = $1
        ORDER BY p.created_at DESC
        "#,
    )
    .bind(user_id.to_string())
    .fetch_all(pool)
    .await?;

    rows_to_projects(rows)
}

pub async fn find_all_for_user_paginated(
    pool: &SqlitePool,
    user_id: Uuid,
    limit: i64,
    offset: i64,
) -> AppResult<(Vec<Project>, i64)> {
    let total: (i64,) = sqlx::query_as(
        r#"
        SELECT COUNT(*)
        FROM projects p
        INNER JOIN project_members pm ON p.id = pm.project_id
        WHERE pm.user_id = $1
        "#,
    )
    .bind(user_id.to_string())
    .fetch_one(pool)
    .await?;

    let rows = sqlx::query_as::<_, ProjectRow>(
        r#"
        SELECT p.id, p.name, p.description, p.repository_path, p.created_at, p.updated_at
        FROM projects p
        INNER JOIN project_members pm ON p.id = pm.project_id
        WHERE pm.user_id = $1
        ORDER BY p.created_at DESC
        LIMIT $2 OFFSET $3
        "#,
    )
    .bind(user_id.to_string())
    .bind(limit)
    .bind(offset)
    .fetch_all(pool)
    .await?;

    let projects = rows_to_projects(rows)?;
    Ok((projects, total.0))
}

pub async fn update(
    pool: &SqlitePool,
    id: Uuid,
    name: &str,
    description: Option<&str>,
    repository_path: Option<&str>,
    now: DateTime<Utc>,
) -> AppResult<()> {
    sqlx::query(
        r#"
        UPDATE projects
        SET name = $1, description = $2, repository_path = $3, updated_at = $4
        WHERE id = $5
        "#,
    )
    .bind(name)
    .bind(description)
    .bind(repository_path)
    .bind(now)
    .bind(id.to_string())
    .execute(pool)
    .await?;

    Ok(())
}

pub async fn delete(pool: &SqlitePool, id: Uuid) -> AppResult<u64> {
    let result = sqlx::query(
        r#"
        DELETE FROM projects
        WHERE id = $1
        "#,
    )
    .bind(id.to_string())
    .execute(pool)
    .await?;

    Ok(result.rows_affected())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::test_helpers::setup_test_db;

    async fn insert_test_project(pool: &SqlitePool, name: &str) -> Uuid {
        let id = Uuid::new_v4();
        let now = Utc::now();
        insert(pool, id, name, None, None, now)
            .await
            .expect("insert project");
        id
    }

    async fn insert_test_user(pool: &SqlitePool) -> Uuid {
        let id = Uuid::new_v4();
        let now = Utc::now();
        let password_hash = "hashed_password";
        sqlx::query(
            r#"
            INSERT INTO users (id, email, name, password_hash, created_at, updated_at)
            VALUES ($1, $2, $3, $4, $5, $6)
            "#,
        )
        .bind(id.to_string())
        .bind(format!("test-{}@example.com", id))
        .bind("Test User")
        .bind(password_hash)
        .bind(now)
        .bind(now)
        .execute(pool)
        .await
        .expect("insert user");
        id
    }

    async fn insert_project_member(pool: &SqlitePool, project_id: Uuid, user_id: Uuid) {
        let now = Utc::now();
        sqlx::query(
            r#"
            INSERT INTO project_members (project_id, user_id, role, created_at)
            VALUES ($1, $2, $3, $4)
            "#,
        )
        .bind(project_id.to_string())
        .bind(user_id.to_string())
        .bind("member")
        .bind(now)
        .execute(pool)
        .await
        .expect("insert project member");
    }

    #[tokio::test]
    async fn test_insert_and_find_by_id() {
        let pool = setup_test_db().await;
        let id = Uuid::new_v4();
        let now = Utc::now();

        insert(
            &pool,
            id,
            "Test Project",
            Some("A description"),
            Some("/repo/path"),
            now,
        )
        .await
        .expect("insert");

        let project = find_by_id(&pool, id)
            .await
            .expect("find")
            .expect("should exist");

        assert_eq!(project.id, id);
        assert_eq!(project.name, "Test Project");
        assert_eq!(project.description, Some("A description".to_string()));
        assert_eq!(project.repository_path, Some("/repo/path".to_string()));
    }

    #[tokio::test]
    async fn test_find_by_id_returns_none_for_nonexistent() {
        let pool = setup_test_db().await;

        let result = find_by_id(&pool, Uuid::new_v4()).await.expect("find");

        assert!(result.is_none());
    }

    #[tokio::test]
    async fn test_insert_without_optional_fields() {
        let pool = setup_test_db().await;
        let id = Uuid::new_v4();
        let now = Utc::now();

        insert(&pool, id, "Minimal Project", None, None, now)
            .await
            .expect("insert");

        let project = find_by_id(&pool, id)
            .await
            .expect("find")
            .expect("should exist");

        assert_eq!(project.name, "Minimal Project");
        assert!(project.description.is_none());
        assert!(project.repository_path.is_none());
    }

    #[tokio::test]
    async fn test_find_all_empty() {
        let pool = setup_test_db().await;

        let projects = find_all(&pool).await.expect("find_all");

        assert!(projects.is_empty());
    }

    #[tokio::test]
    async fn test_find_all_returns_multiple() {
        let pool = setup_test_db().await;
        insert_test_project(&pool, "Project A").await;
        insert_test_project(&pool, "Project B").await;

        let projects = find_all(&pool).await.expect("find_all");

        assert_eq!(projects.len(), 2);
    }

    #[tokio::test]
    async fn test_find_all_paginated() {
        let pool = setup_test_db().await;
        for i in 0..5 {
            insert_test_project(&pool, &format!("Project {i}")).await;
        }

        let (projects, total) = find_all_paginated(&pool, 2, 0)
            .await
            .expect("find_all_paginated");

        assert_eq!(projects.len(), 2);
        assert_eq!(total, 5);
    }

    #[tokio::test]
    async fn test_find_all_paginated_with_offset() {
        let pool = setup_test_db().await;
        for i in 0..5 {
            insert_test_project(&pool, &format!("Project {i}")).await;
        }

        let (projects, total) = find_all_paginated(&pool, 2, 3)
            .await
            .expect("find_all_paginated");

        assert_eq!(projects.len(), 2);
        assert_eq!(total, 5);
    }

    #[tokio::test]
    async fn test_find_all_for_user() {
        let pool = setup_test_db().await;
        let user_id = insert_test_user(&pool).await;
        let p1 = insert_test_project(&pool, "Project 1").await;
        let _p2 = insert_test_project(&pool, "Project 2").await;

        insert_project_member(&pool, p1, user_id).await;

        let projects = find_all_for_user(&pool, user_id)
            .await
            .expect("find_all_for_user");

        assert_eq!(projects.len(), 1);
        assert_eq!(projects[0].id, p1);
    }

    #[tokio::test]
    async fn test_find_all_for_user_paginated() {
        let pool = setup_test_db().await;
        let user_id = insert_test_user(&pool).await;

        for i in 0..5 {
            let pid = insert_test_project(&pool, &format!("Project {i}")).await;
            insert_project_member(&pool, pid, user_id).await;
        }

        let (projects, total) = find_all_for_user_paginated(&pool, user_id, 2, 0)
            .await
            .expect("find_all_for_user_paginated");

        assert_eq!(projects.len(), 2);
        assert_eq!(total, 5);
    }

    #[tokio::test]
    async fn test_update() {
        let pool = setup_test_db().await;
        let id = insert_test_project(&pool, "Original").await;

        let now = Utc::now();
        update(
            &pool,
            id,
            "Updated",
            Some("New desc"),
            Some("/new/path"),
            now,
        )
        .await
        .expect("update");

        let project = find_by_id(&pool, id)
            .await
            .expect("find")
            .expect("should exist");

        assert_eq!(project.name, "Updated");
        assert_eq!(project.description, Some("New desc".to_string()));
        assert_eq!(project.repository_path, Some("/new/path".to_string()));
    }

    #[tokio::test]
    async fn test_delete_existing() {
        let pool = setup_test_db().await;
        let id = insert_test_project(&pool, "To Delete").await;

        let rows = delete(&pool, id).await.expect("delete");

        assert_eq!(rows, 1);
        assert!(find_by_id(&pool, id).await.expect("find").is_none());
    }

    #[tokio::test]
    async fn test_delete_nonexistent() {
        let pool = setup_test_db().await;

        let rows = delete(&pool, Uuid::new_v4()).await.expect("delete");

        assert_eq!(rows, 0);
    }
}
