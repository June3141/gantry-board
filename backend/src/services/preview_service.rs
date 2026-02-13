use chrono::{DateTime, Utc};
use sqlx::prelude::FromRow;
use sqlx::{SqliteConnection, SqlitePool};
use uuid::Uuid;

use crate::error::{AppError, AppResult};
use crate::models::docker_preview::{DockerPreview, PreviewStatus};

#[derive(FromRow)]
struct DockerPreviewRow {
    id: String,
    worktree_name: String,
    container_id: Option<String>,
    port: Option<i32>,
    status: PreviewStatus,
    preview_url: Option<String>,
    error_message: Option<String>,
    created_at: DateTime<Utc>,
    updated_at: DateTime<Utc>,
}

impl TryFrom<DockerPreviewRow> for DockerPreview {
    type Error = uuid::Error;

    fn try_from(row: DockerPreviewRow) -> Result<Self, Self::Error> {
        Ok(DockerPreview {
            id: row.id.parse()?,
            worktree_name: row.worktree_name,
            container_id: row.container_id,
            port: row.port,
            status: row.status,
            preview_url: row.preview_url,
            error_message: row.error_message,
            created_at: row.created_at,
            updated_at: row.updated_at,
        })
    }
}

pub async fn create_preview(pool: &SqlitePool, worktree_name: &str) -> AppResult<DockerPreview> {
    let id = Uuid::new_v4();
    let now = Utc::now();

    sqlx::query(
        r#"
        INSERT INTO docker_previews (id, worktree_name, status, created_at, updated_at)
        VALUES ($1, $2, $3, $4, $5)
        "#,
    )
    .bind(id.to_string())
    .bind(worktree_name)
    .bind(PreviewStatus::Pending)
    .bind(now)
    .bind(now)
    .execute(pool)
    .await?;

    get_preview(pool, id).await
}

pub async fn get_preview(pool: &SqlitePool, id: Uuid) -> AppResult<DockerPreview> {
    let row = sqlx::query_as::<_, DockerPreviewRow>(
        "SELECT id, worktree_name, container_id, port, status, preview_url, error_message, created_at, updated_at FROM docker_previews WHERE id = $1",
    )
    .bind(id.to_string())
    .fetch_optional(pool)
    .await?;

    row.map(|r| r.try_into())
        .transpose()
        .map_err(|e: uuid::Error| AppError::Internal(e.to_string()))?
        .ok_or_else(|| AppError::NotFound(format!("preview {id} not found")))
}

pub async fn list_previews(pool: &SqlitePool) -> AppResult<Vec<DockerPreview>> {
    let rows = sqlx::query_as::<_, DockerPreviewRow>(
        "SELECT id, worktree_name, container_id, port, status, preview_url, error_message, created_at, updated_at FROM docker_previews ORDER BY created_at DESC",
    )
    .fetch_all(pool)
    .await?;

    rows.into_iter()
        .map(|r| {
            r.try_into()
                .map_err(|e: uuid::Error| AppError::Internal(e.to_string()))
        })
        .collect()
}

pub async fn delete_preview(pool: &SqlitePool, id: Uuid) -> AppResult<()> {
    let result = sqlx::query("DELETE FROM docker_previews WHERE id = $1")
        .bind(id.to_string())
        .execute(pool)
        .await?;

    if result.rows_affected() == 0 {
        return Err(AppError::NotFound(format!("preview {id} not found")));
    }

    Ok(())
}

pub async fn update_status_tx(
    conn: &mut SqliteConnection,
    id: Uuid,
    status: PreviewStatus,
    error_message: Option<String>,
) -> AppResult<DockerPreview> {
    let now = Utc::now();
    let result = sqlx::query(
        r#"
        UPDATE docker_previews
        SET status = $1, error_message = $2, updated_at = $3
        WHERE id = $4
        "#,
    )
    .bind(&status)
    .bind(&error_message)
    .bind(now)
    .bind(id.to_string())
    .execute(&mut *conn)
    .await?;

    if result.rows_affected() == 0 {
        return Err(AppError::NotFound(format!("preview {id} not found")));
    }

    let row = sqlx::query_as::<_, DockerPreviewRow>(
        "SELECT id, worktree_name, container_id, port, status, preview_url, error_message, created_at, updated_at FROM docker_previews WHERE id = $1",
    )
    .bind(id.to_string())
    .fetch_one(&mut *conn)
    .await?;

    row.try_into()
        .map_err(|e: uuid::Error| AppError::Internal(e.to_string()))
}

pub async fn update_container_info_tx(
    conn: &mut SqliteConnection,
    id: Uuid,
    container_id: &str,
    port: i32,
    preview_url: &str,
) -> AppResult<DockerPreview> {
    let now = Utc::now();
    let result = sqlx::query(
        r#"
        UPDATE docker_previews
        SET container_id = $1, port = $2, preview_url = $3, updated_at = $4
        WHERE id = $5
        "#,
    )
    .bind(container_id)
    .bind(port)
    .bind(preview_url)
    .bind(now)
    .bind(id.to_string())
    .execute(&mut *conn)
    .await?;

    if result.rows_affected() == 0 {
        return Err(AppError::NotFound(format!("preview {id} not found")));
    }

    let row = sqlx::query_as::<_, DockerPreviewRow>(
        "SELECT id, worktree_name, container_id, port, status, preview_url, error_message, created_at, updated_at FROM docker_previews WHERE id = $1",
    )
    .bind(id.to_string())
    .fetch_one(&mut *conn)
    .await?;

    row.try_into()
        .map_err(|e: uuid::Error| AppError::Internal(e.to_string()))
}

pub async fn find_available_port(
    pool: &SqlitePool,
    range_start: u16,
    range_end: u16,
) -> AppResult<i32> {
    let allocated: Vec<(i32,)> = sqlx::query_as(
        "SELECT port FROM docker_previews WHERE port IS NOT NULL AND status IN ('pending', 'building', 'running')",
    )
    .fetch_all(pool)
    .await?;

    let allocated_ports: std::collections::HashSet<i32> =
        allocated.into_iter().map(|(p,)| p).collect();

    for port in i32::from(range_start)..=i32::from(range_end) {
        if !allocated_ports.contains(&port) {
            return Ok(port);
        }
    }

    Err(AppError::Conflict(
        "no available ports in configured range".to_string(),
    ))
}
