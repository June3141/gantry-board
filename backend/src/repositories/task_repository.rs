use chrono::{DateTime, Utc};
use sqlx::prelude::FromRow;
use sqlx::sqlite::SqliteConnection;
use sqlx::SqlitePool;
use uuid::Uuid;

use crate::error::{AppError, AppResult};
use crate::models::task::{Task, TaskPriority, TaskStatus};

#[derive(FromRow)]
pub(crate) struct TaskRow {
    pub id: String,
    pub project_id: String,
    pub title: String,
    pub description: Option<String>,
    pub status: TaskStatus,
    pub priority: TaskPriority,
    pub parent_id: Option<String>,
    pub assigned_to: Option<String>,
    pub position: i32,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

impl TryFrom<TaskRow> for Task {
    type Error = uuid::Error;

    fn try_from(row: TaskRow) -> Result<Self, Self::Error> {
        Ok(Task {
            id: row.id.parse()?,
            project_id: row.project_id.parse()?,
            title: row.title,
            description: row.description,
            status: row.status,
            priority: row.priority,
            parent_id: row.parent_id.map(|s| s.parse()).transpose()?,
            assigned_to: row.assigned_to.map(|s| s.parse()).transpose()?,
            position: row.position,
            created_at: row.created_at,
            updated_at: row.updated_at,
        })
    }
}

fn row_to_task(row: TaskRow) -> AppResult<Task> {
    row.try_into()
        .map_err(|e: uuid::Error| AppError::Internal(e.to_string()))
}

pub async fn insert_tx(
    conn: &mut SqliteConnection,
    id: Uuid,
    project_id: Uuid,
    title: &str,
    description: Option<&str>,
    status: &TaskStatus,
    priority: &TaskPriority,
    parent_id: Option<Uuid>,
    assigned_to: Option<Uuid>,
    now: DateTime<Utc>,
) -> AppResult<()> {
    sqlx::query(
        r#"
        INSERT INTO tasks (id, project_id, title, description, status, priority, parent_id, assigned_to, position, created_at, updated_at)
        VALUES ($1, $2, $3, $4, $5, $6, $7, $8, $9, $10, $11)
        "#,
    )
    .bind(id.to_string())
    .bind(project_id.to_string())
    .bind(title)
    .bind(description)
    .bind(status)
    .bind(priority)
    .bind(parent_id.map(|u| u.to_string()))
    .bind(assigned_to.map(|u| u.to_string()))
    .bind(0i32)
    .bind(now)
    .bind(now)
    .execute(&mut *conn)
    .await?;

    Ok(())
}

pub async fn find_by_id(pool: &SqlitePool, id: Uuid) -> AppResult<Task> {
    let row = sqlx::query_as::<_, TaskRow>(
        r#"
        SELECT id, project_id, title, description, status, priority, parent_id, assigned_to, position, created_at, updated_at
        FROM tasks
        WHERE id = $1
        "#,
    )
    .bind(id.to_string())
    .fetch_optional(pool)
    .await?;

    row.map(row_to_task)
        .transpose()?
        .ok_or_else(|| AppError::NotFound(format!("task {} not found", id)))
}

pub async fn find_by_id_tx(conn: &mut SqliteConnection, id: Uuid) -> AppResult<Task> {
    let row = sqlx::query_as::<_, TaskRow>(
        r#"
        SELECT id, project_id, title, description, status, priority, parent_id, assigned_to, position, created_at, updated_at
        FROM tasks
        WHERE id = $1
        "#,
    )
    .bind(id.to_string())
    .fetch_optional(&mut *conn)
    .await?;

    row.map(row_to_task)
        .transpose()?
        .ok_or_else(|| AppError::NotFound(format!("task {} not found", id)))
}

pub async fn find_all_by_project(pool: &SqlitePool, project_id: Uuid) -> AppResult<Vec<Task>> {
    let rows = sqlx::query_as::<_, TaskRow>(
        r#"
        SELECT id, project_id, title, description, status, priority, parent_id, assigned_to, position, created_at, updated_at
        FROM tasks
        WHERE project_id = $1
        ORDER BY position ASC, created_at ASC
        "#,
    )
    .bind(project_id.to_string())
    .fetch_all(pool)
    .await?;

    rows.into_iter().map(row_to_task).collect()
}

pub async fn count_by_project(pool: &SqlitePool, project_id: Uuid) -> AppResult<i64> {
    let total: (i64,) = sqlx::query_as("SELECT COUNT(*) FROM tasks WHERE project_id = $1")
        .bind(project_id.to_string())
        .fetch_one(pool)
        .await?;

    Ok(total.0)
}

pub async fn find_paginated_by_project(
    pool: &SqlitePool,
    project_id: Uuid,
    limit: i64,
    offset: i64,
) -> AppResult<Vec<Task>> {
    let rows = sqlx::query_as::<_, TaskRow>(
        r#"
        SELECT id, project_id, title, description, status, priority, parent_id, assigned_to, position, created_at, updated_at
        FROM tasks
        WHERE project_id = $1
        ORDER BY position ASC, created_at ASC
        LIMIT $2 OFFSET $3
        "#,
    )
    .bind(project_id.to_string())
    .bind(limit)
    .bind(offset)
    .fetch_all(pool)
    .await?;

    rows.into_iter().map(row_to_task).collect()
}

pub async fn update_tx(
    conn: &mut SqliteConnection,
    id: Uuid,
    title: &str,
    description: Option<&str>,
    status: &TaskStatus,
    priority: &TaskPriority,
    parent_id: Option<Uuid>,
    assigned_to: Option<Uuid>,
    position: i32,
    now: DateTime<Utc>,
) -> AppResult<()> {
    sqlx::query(
        r#"
        UPDATE tasks
        SET title = $1, description = $2, status = $3, priority = $4, parent_id = $5, assigned_to = $6, position = $7, updated_at = $8
        WHERE id = $9
        "#,
    )
    .bind(title)
    .bind(description)
    .bind(status)
    .bind(priority)
    .bind(parent_id.map(|u| u.to_string()))
    .bind(assigned_to.map(|u| u.to_string()))
    .bind(position)
    .bind(now)
    .bind(id.to_string())
    .execute(&mut *conn)
    .await?;

    Ok(())
}

pub async fn delete(pool: &SqlitePool, id: Uuid) -> AppResult<()> {
    let result = sqlx::query("DELETE FROM tasks WHERE id = $1")
        .bind(id.to_string())
        .execute(pool)
        .await?;

    if result.rows_affected() == 0 {
        return Err(AppError::NotFound(format!("task {} not found", id)));
    }

    Ok(())
}

pub async fn user_exists_tx(conn: &mut SqliteConnection, user_id: Uuid) -> AppResult<bool> {
    let exists: Option<(i32,)> = sqlx::query_as("SELECT 1 FROM users WHERE id = $1")
        .bind(user_id.to_string())
        .fetch_optional(&mut *conn)
        .await?;

    Ok(exists.is_some())
}

pub async fn is_project_member_tx(
    conn: &mut SqliteConnection,
    project_id: Uuid,
    user_id: Uuid,
) -> AppResult<bool> {
    let exists: Option<(i32,)> =
        sqlx::query_as("SELECT 1 FROM project_members WHERE project_id = $1 AND user_id = $2")
            .bind(project_id.to_string())
            .bind(user_id.to_string())
            .fetch_optional(&mut *conn)
            .await?;

    Ok(exists.is_some())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::models::project::{AddMemberRequest, CreateProjectRequest, MemberRole};
    use crate::models::user::RegisterRequest;
    use crate::services::{member_service, project_service, user_service};
    use crate::test_helpers::setup_test_db;

    async fn create_test_project(pool: &SqlitePool) -> Uuid {
        let req = CreateProjectRequest {
            name: "Test Project".to_string(),
            description: None,
            repository_path: None,
        };
        project_service::create_project(pool, &req)
            .await
            .expect("create project")
            .id
    }

    async fn create_test_user(pool: &SqlitePool) -> Uuid {
        let req = RegisterRequest {
            email: format!("test-{}@example.com", Uuid::new_v4()),
            name: "Test User".to_string(),
            password: "correct horse battery staple purple".to_string(),
        };
        user_service::create_user(pool, &req)
            .await
            .expect("create user")
            .id
    }

    #[tokio::test]
    async fn test_insert_and_find_by_id() {
        let pool = setup_test_db().await;
        let project_id = create_test_project(&pool).await;

        let id = Uuid::new_v4();
        let now = Utc::now();

        let mut tx = pool.begin().await.unwrap();
        insert_tx(
            &mut *tx,
            id,
            project_id,
            "Test Task",
            Some("desc"),
            &TaskStatus::Backlog,
            &TaskPriority::Medium,
            None,
            None,
            now,
        )
        .await
        .expect("insert");
        tx.commit().await.unwrap();

        let task = find_by_id(&pool, id).await.expect("find");
        assert_eq!(task.id, id);
        assert_eq!(task.title, "Test Task");
        assert_eq!(task.description, Some("desc".to_string()));
    }

    #[tokio::test]
    async fn test_find_by_id_not_found() {
        let pool = setup_test_db().await;
        let result = find_by_id(&pool, Uuid::new_v4()).await;
        assert!(matches!(result, Err(AppError::NotFound(_))));
    }

    #[tokio::test]
    async fn test_find_all_by_project() {
        let pool = setup_test_db().await;
        let project_id = create_test_project(&pool).await;
        let now = Utc::now();

        for i in 0..3 {
            let mut tx = pool.begin().await.unwrap();
            insert_tx(
                &mut *tx,
                Uuid::new_v4(),
                project_id,
                &format!("Task {i}"),
                None,
                &TaskStatus::Backlog,
                &TaskPriority::Medium,
                None,
                None,
                now,
            )
            .await
            .expect("insert");
            tx.commit().await.unwrap();
        }

        let tasks = find_all_by_project(&pool, project_id)
            .await
            .expect("find all");
        assert_eq!(tasks.len(), 3);
    }

    #[tokio::test]
    async fn test_count_and_paginated() {
        let pool = setup_test_db().await;
        let project_id = create_test_project(&pool).await;
        let now = Utc::now();

        for i in 0..5 {
            let mut tx = pool.begin().await.unwrap();
            insert_tx(
                &mut *tx,
                Uuid::new_v4(),
                project_id,
                &format!("Task {i}"),
                None,
                &TaskStatus::Backlog,
                &TaskPriority::Medium,
                None,
                None,
                now,
            )
            .await
            .expect("insert");
            tx.commit().await.unwrap();
        }

        let count = count_by_project(&pool, project_id).await.expect("count");
        assert_eq!(count, 5);

        let page = find_paginated_by_project(&pool, project_id, 2, 0)
            .await
            .expect("paginated");
        assert_eq!(page.len(), 2);
    }

    #[tokio::test]
    async fn test_update_tx() {
        let pool = setup_test_db().await;
        let project_id = create_test_project(&pool).await;
        let id = Uuid::new_v4();
        let now = Utc::now();

        let mut tx = pool.begin().await.unwrap();
        insert_tx(
            &mut *tx,
            id,
            project_id,
            "Original",
            None,
            &TaskStatus::Backlog,
            &TaskPriority::Low,
            None,
            None,
            now,
        )
        .await
        .expect("insert");
        tx.commit().await.unwrap();

        let later = now + chrono::Duration::seconds(10);
        let mut tx = pool.begin().await.unwrap();
        update_tx(
            &mut *tx,
            id,
            "Updated",
            Some("new desc"),
            &TaskStatus::InProgress,
            &TaskPriority::High,
            None,
            None,
            5,
            later,
        )
        .await
        .expect("update");
        tx.commit().await.unwrap();

        let task = find_by_id(&pool, id).await.expect("find");
        assert_eq!(task.title, "Updated");
        assert_eq!(task.description, Some("new desc".to_string()));
        assert!(matches!(task.status, TaskStatus::InProgress));
        assert_eq!(task.position, 5);
    }

    #[tokio::test]
    async fn test_delete() {
        let pool = setup_test_db().await;
        let project_id = create_test_project(&pool).await;
        let id = Uuid::new_v4();
        let now = Utc::now();

        let mut tx = pool.begin().await.unwrap();
        insert_tx(
            &mut *tx,
            id,
            project_id,
            "To Delete",
            None,
            &TaskStatus::Backlog,
            &TaskPriority::Medium,
            None,
            None,
            now,
        )
        .await
        .expect("insert");
        tx.commit().await.unwrap();

        delete(&pool, id).await.expect("delete");
        assert!(matches!(
            find_by_id(&pool, id).await,
            Err(AppError::NotFound(_))
        ));
    }

    #[tokio::test]
    async fn test_delete_not_found() {
        let pool = setup_test_db().await;
        let result = delete(&pool, Uuid::new_v4()).await;
        assert!(matches!(result, Err(AppError::NotFound(_))));
    }

    #[tokio::test]
    async fn test_user_exists_tx() {
        let pool = setup_test_db().await;
        let user_id = create_test_user(&pool).await;

        let mut tx = pool.begin().await.unwrap();
        assert!(user_exists_tx(&mut *tx, user_id).await.expect("check"));
        assert!(!user_exists_tx(&mut *tx, Uuid::new_v4())
            .await
            .expect("check"));
        tx.commit().await.unwrap();
    }

    #[tokio::test]
    async fn test_is_project_member_tx() {
        let pool = setup_test_db().await;
        let project_id = create_test_project(&pool).await;
        let user_id = create_test_user(&pool).await;

        let req = AddMemberRequest {
            user_id,
            role: MemberRole::Member,
        };
        member_service::add_member(&pool, project_id, &req)
            .await
            .expect("add member");

        let mut tx = pool.begin().await.unwrap();
        assert!(is_project_member_tx(&mut *tx, project_id, user_id)
            .await
            .expect("check"));
        assert!(!is_project_member_tx(&mut *tx, project_id, Uuid::new_v4())
            .await
            .expect("check"));
        tx.commit().await.unwrap();
    }
}
