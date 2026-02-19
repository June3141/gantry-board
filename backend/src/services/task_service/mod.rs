use chrono::{DateTime, Utc};
use sqlx::prelude::FromRow;
use sqlx::sqlite::SqliteConnection;
use sqlx::SqlitePool;
use uuid::Uuid;

use crate::error::{AppError, AppResult};
use crate::models::task::{CreateTaskRequest, Task, TaskPriority, TaskStatus, UpdateTaskRequest};

#[cfg(test)]
mod tests;

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

#[tracing::instrument(skip(pool, req), fields(project_id = %req.project_id))]
#[allow(clippy::explicit_auto_deref)] // sqlx Transaction requires explicit deref
pub async fn create_task(pool: &SqlitePool, req: &CreateTaskRequest) -> AppResult<Task> {
    let mut tx = pool.begin().await?;

    validate_assigned_to_tx(&mut *tx, req.assigned_to, req.project_id).await?;
    validate_parent_project_tx(&mut *tx, req.parent_id, req.project_id).await?;

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
    .execute(&mut *tx)
    .await?;

    tx.commit().await?;

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
    .await?;

    row.map(|r| r.try_into())
        .transpose()
        .map_err(|e: uuid::Error| AppError::Internal(e.to_string()))?
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
    .await?;

    rows.into_iter()
        .map(|r| r.try_into())
        .collect::<Result<Vec<_>, _>>()
        .map_err(|e: uuid::Error| AppError::Internal(e.to_string()))
}

pub async fn list_tasks_paginated(
    pool: &SqlitePool,
    project_id: Uuid,
    limit: i64,
    offset: i64,
) -> AppResult<(Vec<Task>, i64)> {
    let total: (i64,) = sqlx::query_as(
        r#"
        SELECT COUNT(*) FROM tasks WHERE project_id = $1
        "#,
    )
    .bind(project_id.to_string())
    .fetch_one(pool)
    .await?;

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

    let tasks = rows
        .into_iter()
        .map(|r| r.try_into())
        .collect::<Result<Vec<_>, _>>()
        .map_err(|e: uuid::Error| AppError::Internal(e.to_string()))?;

    Ok((tasks, total.0))
}

#[tracing::instrument(skip(pool, req), fields(task_id = %id))]
#[allow(clippy::explicit_auto_deref)] // sqlx Transaction requires explicit deref
pub async fn update_task(pool: &SqlitePool, id: Uuid, req: &UpdateTaskRequest) -> AppResult<Task> {
    let mut tx = pool.begin().await?;

    let existing = get_task_tx(&mut *tx, id).await?;

    validate_assigned_to_tx(&mut *tx, req.assigned_to, existing.project_id).await?;
    validate_parent_project_tx(&mut *tx, req.parent_id, existing.project_id).await?;

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
    .execute(&mut *tx)
    .await?;

    tx.commit().await?;

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

async fn get_task_tx(conn: &mut SqliteConnection, id: Uuid) -> AppResult<Task> {
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

    row.map(|r| r.try_into())
        .transpose()
        .map_err(|e: uuid::Error| AppError::Internal(e.to_string()))?
        .ok_or_else(|| AppError::NotFound(format!("task {} not found", id)))
}

async fn validate_assigned_to_tx(
    conn: &mut SqliteConnection,
    assigned_to: Option<Uuid>,
    project_id: Uuid,
) -> AppResult<()> {
    if let Some(user_id) = assigned_to {
        let exists: Option<(i32,)> = sqlx::query_as("SELECT 1 FROM users WHERE id = $1")
            .bind(user_id.to_string())
            .fetch_optional(&mut *conn)
            .await?;
        if exists.is_none() {
            return Err(AppError::Validation(format!(
                "assigned user {} does not exist",
                user_id
            )));
        }

        let is_member: Option<(i32,)> =
            sqlx::query_as("SELECT 1 FROM project_members WHERE project_id = $1 AND user_id = $2")
                .bind(project_id.to_string())
                .bind(user_id.to_string())
                .fetch_optional(&mut *conn)
                .await?;
        if is_member.is_none() {
            return Err(AppError::Validation(format!(
                "assigned user {} is not a member of project {}",
                user_id, project_id
            )));
        }
    }
    Ok(())
}

async fn validate_parent_project_tx(
    conn: &mut SqliteConnection,
    parent_id: Option<Uuid>,
    project_id: Uuid,
) -> AppResult<()> {
    if let Some(pid) = parent_id {
        let parent = get_task_tx(&mut *conn, pid).await.map_err(|e| match e {
            AppError::NotFound(_) => {
                AppError::Validation(format!("parent task {} does not exist", pid))
            }
            other => other,
        })?;
        if parent.project_id != project_id {
            return Err(AppError::Validation(
                "parent task must belong to the same project".to_string(),
            ));
        }
    }
    Ok(())
}

#[tracing::instrument(skip(pool), fields(task_id = %id))]
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
