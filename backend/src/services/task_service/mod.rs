use sqlx::sqlite::SqliteConnection;
use sqlx::SqlitePool;
use uuid::Uuid;

use crate::error::{AppError, AppResult};
use crate::models::task::{CreateTaskRequest, Task, TaskPriority, TaskStatus, UpdateTaskRequest};
use crate::repositories::task_repository;

#[cfg(test)]
mod tests;

#[tracing::instrument(skip(pool, req), fields(project_id = %req.project_id))]
#[allow(clippy::explicit_auto_deref)] // sqlx Transaction requires explicit deref
pub async fn create_task(pool: &SqlitePool, req: &CreateTaskRequest) -> AppResult<Task> {
    let mut tx = pool.begin().await?;

    validate_assigned_to_tx(&mut tx, req.assigned_to, req.project_id).await?;
    validate_parent_project_tx(&mut tx, req.parent_id, req.project_id).await?;

    let id = uuid::Uuid::new_v4();
    let now = chrono::Utc::now();
    let status = req.status.clone().unwrap_or(TaskStatus::Backlog);
    let priority = req.priority.clone().unwrap_or(TaskPriority::Medium);

    task_repository::insert_tx(
        &mut tx,
        id,
        req.project_id,
        &req.title,
        req.description.as_deref(),
        &status,
        &priority,
        req.parent_id,
        req.assigned_to,
        now,
    )
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
    task_repository::find_by_id(pool, id).await
}

pub async fn list_tasks(pool: &SqlitePool, project_id: Uuid) -> AppResult<Vec<Task>> {
    task_repository::find_all_by_project(pool, project_id).await
}

pub async fn list_tasks_paginated(
    pool: &SqlitePool,
    project_id: Uuid,
    limit: i64,
    offset: i64,
) -> AppResult<(Vec<Task>, i64)> {
    let total = task_repository::count_by_project(pool, project_id).await?;
    let tasks = task_repository::find_paginated_by_project(pool, project_id, limit, offset).await?;

    Ok((tasks, total))
}

#[tracing::instrument(skip(pool, req), fields(task_id = %id))]
#[allow(clippy::explicit_auto_deref)] // sqlx Transaction requires explicit deref
pub async fn update_task(pool: &SqlitePool, id: Uuid, req: &UpdateTaskRequest) -> AppResult<Task> {
    let mut tx = pool.begin().await?;

    let existing = task_repository::find_by_id_tx(&mut tx, id).await?;

    validate_assigned_to_tx(&mut tx, req.assigned_to, existing.project_id).await?;
    validate_parent_project_tx(&mut tx, req.parent_id, existing.project_id).await?;

    let now = chrono::Utc::now();

    let title = req.title.as_ref().unwrap_or(&existing.title);
    let description = req.description.as_ref().or(existing.description.as_ref());
    let status = req.status.as_ref().unwrap_or(&existing.status);
    let priority = req.priority.as_ref().unwrap_or(&existing.priority);
    let parent_id = req.parent_id.or(existing.parent_id);
    let assigned_to = req.assigned_to.or(existing.assigned_to);
    let position = req.position.unwrap_or(existing.position);

    // Track status change for metrics
    let status_changed = *status != existing.status;

    task_repository::update_tx(
        &mut tx,
        id,
        title,
        description.map(|s| s.as_str()),
        status,
        priority,
        parent_id,
        assigned_to,
        position,
        now,
    )
    .await?;

    tx.commit().await?;

    // Record task status change metric after successful commit
    if status_changed {
        let status_label = serde_json::to_string(status)
            .unwrap_or_else(|_| "unknown".to_string())
            .trim_matches('"')
            .to_string();
        metrics::counter!(
            crate::observability::metric::TASKS_TOTAL,
            "status" => status_label,
        )
        .increment(1);
    }

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

async fn validate_assigned_to_tx(
    conn: &mut SqliteConnection,
    assigned_to: Option<Uuid>,
    project_id: Uuid,
) -> AppResult<()> {
    if let Some(user_id) = assigned_to {
        if !task_repository::user_exists_tx(&mut *conn, user_id).await? {
            return Err(AppError::Validation(format!(
                "assigned user {} does not exist",
                user_id
            )));
        }

        if !task_repository::is_project_member_tx(&mut *conn, project_id, user_id).await? {
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
        let parent = task_repository::find_by_id_tx(&mut *conn, pid)
            .await
            .map_err(|e| match e {
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
    task_repository::delete(pool, id).await
}
