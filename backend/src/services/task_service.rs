use chrono::{DateTime, Utc};
use sqlx::prelude::FromRow;
use sqlx::SqlitePool;
use uuid::Uuid;

use crate::error::{AppError, AppResult};
use crate::models::task::{CreateTaskRequest, Task, TaskPriority, TaskStatus, UpdateTaskRequest};

#[derive(FromRow)]
struct TaskRow {
    id: String,
    project_id: String,
    title: String,
    description: Option<String>,
    status: TaskStatus,
    priority: TaskPriority,
    parent_id: Option<String>,
    assigned_to: Option<String>,
    position: i32,
    created_at: DateTime<Utc>,
    updated_at: DateTime<Utc>,
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

pub async fn create_task(pool: &SqlitePool, req: &CreateTaskRequest) -> AppResult<Task> {
    let id = Uuid::new_v4();
    let now = Utc::now();
    let status = req.status.clone().unwrap_or(TaskStatus::Backlog);
    let priority = req.priority.clone().unwrap_or(TaskPriority::Medium);

    sqlx::query(
        r#"
        INSERT INTO tasks (id, project_id, title, description, status, priority, parent_id, assigned_to, position, created_at, updated_at)
        VALUES ($1, $2, $3, $4, $5, $6, $7, $8, $9, $10, $11)
        "#,
    )
    .bind(id.to_string())
    .bind(req.project_id.to_string())
    .bind(&req.title)
    .bind(&req.description)
    .bind(&status)
    .bind(&priority)
    .bind(req.parent_id.map(|u| u.to_string()))
    .bind(req.assigned_to.map(|u| u.to_string()))
    .bind(0i32)
    .bind(now)
    .bind(now)
    .execute(pool)
    .await
    ?;

    Ok(Task {
        id,
        project_id: req.project_id,
        title: req.title.clone(),
        description: req.description.clone(),
        status,
        priority,
        parent_id: req.parent_id,
        assigned_to: req.assigned_to,
        position: 0,
        created_at: now,
        updated_at: now,
    })
}

pub async fn get_task(pool: &SqlitePool, id: Uuid) -> AppResult<Task> {
    let row = sqlx::query_as::<_, TaskRow>(
        r#"
        SELECT id, project_id, title, description, status, priority, parent_id, assigned_to, position, created_at, updated_at
        FROM tasks
        WHERE id = $1
        "#,
    )
    .bind(id.to_string())
    .fetch_optional(pool)
    .await
    ?;

    row.map(|r| r.try_into())
        .transpose()
        .map_err(|e: uuid::Error| AppError::Internal(e.into()))?
        .ok_or_else(|| AppError::NotFound(format!("task {} not found", id)))
}

pub async fn list_tasks(pool: &SqlitePool, project_id: Uuid) -> AppResult<Vec<Task>> {
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
    .await
    ?;

    rows.into_iter()
        .map(|r| r.try_into())
        .collect::<Result<Vec<_>, _>>()
        .map_err(|e: uuid::Error| AppError::Internal(e.into()))
}

pub async fn update_task(pool: &SqlitePool, id: Uuid, req: &UpdateTaskRequest) -> AppResult<Task> {
    let existing = get_task(pool, id).await?;
    let now = Utc::now();

    let title = req.title.as_ref().unwrap_or(&existing.title);
    let description = req.description.as_ref().or(existing.description.as_ref());
    let status = req.status.as_ref().unwrap_or(&existing.status);
    let priority = req.priority.as_ref().unwrap_or(&existing.priority);
    let parent_id = req.parent_id.or(existing.parent_id);
    let assigned_to = req.assigned_to.or(existing.assigned_to);
    let position = req.position.unwrap_or(existing.position);

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
    .execute(pool)
    .await
    ?;

    Ok(Task {
        id,
        project_id: existing.project_id,
        title: title.clone(),
        description: description.cloned(),
        status: status.clone(),
        priority: priority.clone(),
        parent_id,
        assigned_to,
        position,
        created_at: existing.created_at,
        updated_at: now,
    })
}

pub async fn delete_task(pool: &SqlitePool, id: Uuid) -> AppResult<()> {
    let result = sqlx::query(
        r#"
        DELETE FROM tasks
        WHERE id = $1
        "#,
    )
    .bind(id.to_string())
    .execute(pool)
    .await?;

    if result.rows_affected() == 0 {
        return Err(AppError::NotFound(format!("task {} not found", id)));
    }

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::models::project::CreateProjectRequest;
    use crate::services::project_service;
    use crate::test_helpers::setup_test_db;

    async fn create_test_project(pool: &SqlitePool) -> Uuid {
        let req = CreateProjectRequest {
            name: "Test Project".to_string(),
            description: None,
        };
        let project = project_service::create_project(pool, &req)
            .await
            .expect("Failed to create project");
        project.id
    }

    #[tokio::test]
    async fn test_create_task_saves_to_db_and_returns() {
        let pool = setup_test_db().await;
        let project_id = create_test_project(&pool).await;

        let req = CreateTaskRequest {
            project_id,
            title: "Test Task".to_string(),
            description: Some("A test task".to_string()),
            status: None,
            priority: None,
            parent_id: None,
            assigned_to: None,
        };
        let task = create_task(&pool, &req)
            .await
            .expect("Failed to create task");

        assert_eq!(task.title, "Test Task");
        assert_eq!(task.description, Some("A test task".to_string()));
        assert_eq!(task.project_id, project_id);
        assert!(matches!(task.status, TaskStatus::Backlog));
        assert!(matches!(task.priority, TaskPriority::Medium));
    }

    #[tokio::test]
    async fn test_create_task_with_custom_status_and_priority() {
        let pool = setup_test_db().await;
        let project_id = create_test_project(&pool).await;

        let req = CreateTaskRequest {
            project_id,
            title: "Urgent Task".to_string(),
            description: None,
            status: Some(TaskStatus::InProgress),
            priority: Some(TaskPriority::Urgent),
            parent_id: None,
            assigned_to: None,
        };
        let task = create_task(&pool, &req)
            .await
            .expect("Failed to create task");

        assert!(matches!(task.status, TaskStatus::InProgress));
        assert!(matches!(task.priority, TaskPriority::Urgent));
    }

    #[tokio::test]
    async fn test_get_task_returns_existing_task() {
        let pool = setup_test_db().await;
        let project_id = create_test_project(&pool).await;

        let req = CreateTaskRequest {
            project_id,
            title: "Get Me".to_string(),
            description: None,
            status: None,
            priority: None,
            parent_id: None,
            assigned_to: None,
        };
        let created = create_task(&pool, &req)
            .await
            .expect("Failed to create task");

        let task = get_task(&pool, created.id)
            .await
            .expect("Failed to get task");

        assert_eq!(task.id, created.id);
        assert_eq!(task.title, "Get Me");
    }

    #[tokio::test]
    async fn test_get_task_returns_not_found_for_nonexistent() {
        let pool = setup_test_db().await;
        let random_id = Uuid::new_v4();

        let result = get_task(&pool, random_id).await;

        assert!(matches!(result, Err(AppError::NotFound(_))));
    }

    #[tokio::test]
    async fn test_list_tasks_returns_empty_when_no_tasks() {
        let pool = setup_test_db().await;
        let project_id = create_test_project(&pool).await;

        let tasks = list_tasks(&pool, project_id)
            .await
            .expect("Failed to list tasks");

        assert!(tasks.is_empty());
    }

    #[tokio::test]
    async fn test_list_tasks_returns_tasks_for_project() {
        let pool = setup_test_db().await;
        let project_id = create_test_project(&pool).await;

        create_task(
            &pool,
            &CreateTaskRequest {
                project_id,
                title: "Task 1".to_string(),
                description: None,
                status: None,
                priority: None,
                parent_id: None,
                assigned_to: None,
            },
        )
        .await
        .expect("Failed to create task 1");

        create_task(
            &pool,
            &CreateTaskRequest {
                project_id,
                title: "Task 2".to_string(),
                description: None,
                status: None,
                priority: None,
                parent_id: None,
                assigned_to: None,
            },
        )
        .await
        .expect("Failed to create task 2");

        let tasks = list_tasks(&pool, project_id)
            .await
            .expect("Failed to list tasks");

        assert_eq!(tasks.len(), 2);
    }

    #[tokio::test]
    async fn test_list_tasks_filters_by_project() {
        let pool = setup_test_db().await;
        let project1 = create_test_project(&pool).await;
        let project2 = create_test_project(&pool).await;

        create_task(
            &pool,
            &CreateTaskRequest {
                project_id: project1,
                title: "Project 1 Task".to_string(),
                description: None,
                status: None,
                priority: None,
                parent_id: None,
                assigned_to: None,
            },
        )
        .await
        .expect("Failed to create task");

        create_task(
            &pool,
            &CreateTaskRequest {
                project_id: project2,
                title: "Project 2 Task".to_string(),
                description: None,
                status: None,
                priority: None,
                parent_id: None,
                assigned_to: None,
            },
        )
        .await
        .expect("Failed to create task");

        let tasks1 = list_tasks(&pool, project1).await.expect("Failed to list");
        let tasks2 = list_tasks(&pool, project2).await.expect("Failed to list");

        assert_eq!(tasks1.len(), 1);
        assert_eq!(tasks1[0].title, "Project 1 Task");
        assert_eq!(tasks2.len(), 1);
        assert_eq!(tasks2[0].title, "Project 2 Task");
    }

    #[tokio::test]
    async fn test_update_task_changes_title() {
        let pool = setup_test_db().await;
        let project_id = create_test_project(&pool).await;

        let created = create_task(
            &pool,
            &CreateTaskRequest {
                project_id,
                title: "Original".to_string(),
                description: None,
                status: None,
                priority: None,
                parent_id: None,
                assigned_to: None,
            },
        )
        .await
        .expect("Failed to create task");

        let updated = update_task(
            &pool,
            created.id,
            &UpdateTaskRequest {
                title: Some("Updated".to_string()),
                description: None,
                status: None,
                priority: None,
                parent_id: None,
                assigned_to: None,
                position: None,
            },
        )
        .await
        .expect("Failed to update task");

        assert_eq!(updated.title, "Updated");
    }

    #[tokio::test]
    async fn test_update_task_changes_status() {
        let pool = setup_test_db().await;
        let project_id = create_test_project(&pool).await;

        let created = create_task(
            &pool,
            &CreateTaskRequest {
                project_id,
                title: "Task".to_string(),
                description: None,
                status: Some(TaskStatus::Backlog),
                priority: None,
                parent_id: None,
                assigned_to: None,
            },
        )
        .await
        .expect("Failed to create task");

        let updated = update_task(
            &pool,
            created.id,
            &UpdateTaskRequest {
                title: None,
                description: None,
                status: Some(TaskStatus::Done),
                priority: None,
                parent_id: None,
                assigned_to: None,
                position: None,
            },
        )
        .await
        .expect("Failed to update task");

        assert!(matches!(updated.status, TaskStatus::Done));
    }

    #[tokio::test]
    async fn test_update_task_changes_position() {
        let pool = setup_test_db().await;
        let project_id = create_test_project(&pool).await;

        let created = create_task(
            &pool,
            &CreateTaskRequest {
                project_id,
                title: "Task".to_string(),
                description: None,
                status: None,
                priority: None,
                parent_id: None,
                assigned_to: None,
            },
        )
        .await
        .expect("Failed to create task");

        let updated = update_task(
            &pool,
            created.id,
            &UpdateTaskRequest {
                title: None,
                description: None,
                status: None,
                priority: None,
                parent_id: None,
                assigned_to: None,
                position: Some(5),
            },
        )
        .await
        .expect("Failed to update task");

        assert_eq!(updated.position, 5);
    }

    #[tokio::test]
    async fn test_delete_task_removes_from_db() {
        let pool = setup_test_db().await;
        let project_id = create_test_project(&pool).await;

        let created = create_task(
            &pool,
            &CreateTaskRequest {
                project_id,
                title: "To Delete".to_string(),
                description: None,
                status: None,
                priority: None,
                parent_id: None,
                assigned_to: None,
            },
        )
        .await
        .expect("Failed to create task");

        delete_task(&pool, created.id)
            .await
            .expect("Failed to delete task");

        let result = get_task(&pool, created.id).await;
        assert!(matches!(result, Err(AppError::NotFound(_))));
    }

    #[tokio::test]
    async fn test_delete_nonexistent_task_returns_not_found() {
        let pool = setup_test_db().await;
        let random_id = Uuid::new_v4();

        let result = delete_task(&pool, random_id).await;

        assert!(matches!(result, Err(AppError::NotFound(_))));
    }
}
