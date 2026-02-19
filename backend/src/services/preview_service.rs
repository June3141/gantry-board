use std::collections::HashMap;
use std::path::PathBuf;
use std::sync::atomic::{AtomicU32, Ordering};
use std::sync::Arc;

#[allow(deprecated)]
use bollard::container::{
    Config as ContainerConfig, CreateContainerOptions, RemoveContainerOptions,
    StartContainerOptions, StopContainerOptions,
};
#[allow(deprecated)]
use bollard::image::BuildImageOptions;
use bollard::secret::{HostConfig, PortBinding, PortMap};
use bollard::Docker;
use bytes::Bytes;
use chrono::{DateTime, Utc};
use futures_util::StreamExt;
use http_body_util::Full;
use sqlx::prelude::FromRow;
use sqlx::{SqliteConnection, SqlitePool};
use tokio::sync::Semaphore;
use tracing::info;
use uuid::Uuid;

use crate::config::Config;
use crate::error::{AppError, AppResult};
use crate::models::docker_preview::{DockerPreview, PreviewStatus};
use crate::sse::event::SseEvent;
use crate::sse::hub::SseHub;

/// Circuit breaker for Docker operations.
/// Opens after `FAILURE_THRESHOLD` consecutive failures and stays open
/// for `RECOVERY_WINDOW` seconds before allowing a single probe request.
pub struct DockerCircuitBreaker {
    consecutive_failures: AtomicU32,
    last_failure_epoch: std::sync::atomic::AtomicU64,
}

const FAILURE_THRESHOLD: u32 = 3;
const RECOVERY_WINDOW_SECS: u64 = 30;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CircuitState {
    Closed,
    Open,
    HalfOpen,
}

impl DockerCircuitBreaker {
    pub fn new() -> Self {
        Self {
            consecutive_failures: AtomicU32::new(0),
            last_failure_epoch: std::sync::atomic::AtomicU64::new(0),
        }
    }

    pub fn state(&self) -> CircuitState {
        let failures = self.consecutive_failures.load(Ordering::Relaxed);
        if failures < FAILURE_THRESHOLD {
            return CircuitState::Closed;
        }
        let last = self.last_failure_epoch.load(Ordering::Relaxed);
        let now = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap_or_default()
            .as_secs();
        if now.saturating_sub(last) >= RECOVERY_WINDOW_SECS {
            CircuitState::HalfOpen
        } else {
            CircuitState::Open
        }
    }

    pub fn check(&self) -> AppResult<()> {
        match self.state() {
            CircuitState::Closed | CircuitState::HalfOpen => Ok(()),
            CircuitState::Open => Err(AppError::Internal(
                "Docker operations temporarily unavailable (circuit breaker open)".to_string(),
            )),
        }
    }

    pub fn record_success(&self) {
        self.consecutive_failures.store(0, Ordering::Relaxed);
    }

    pub fn record_failure(&self) {
        self.consecutive_failures.fetch_add(1, Ordering::Relaxed);
        let now = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap_or_default()
            .as_secs();
        self.last_failure_epoch.store(now, Ordering::Relaxed);
    }
}

impl Default for DockerCircuitBreaker {
    fn default() -> Self {
        Self::new()
    }
}

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

pub async fn update_container_id_tx(
    conn: &mut SqliteConnection,
    id: Uuid,
    container_id: &str,
) -> AppResult<()> {
    let now = Utc::now();
    let result =
        sqlx::query("UPDATE docker_previews SET container_id = $1, updated_at = $2 WHERE id = $3")
            .bind(container_id)
            .bind(now)
            .bind(id.to_string())
            .execute(&mut *conn)
            .await?;

    if result.rows_affected() == 0 {
        return Err(AppError::NotFound(format!("preview {id} not found")));
    }

    Ok(())
}

/// Atomically allocate the next available port within the configured range.
/// Uses BEGIN IMMEDIATE to prevent concurrent allocations of the same port.
pub async fn allocate_port_tx(
    conn: &mut SqliteConnection,
    preview_id: Uuid,
    range_start: u16,
    range_end: u16,
    base_url: &str,
) -> AppResult<i32> {
    let allocated: Vec<(i32,)> = sqlx::query_as(
        "SELECT port FROM docker_previews WHERE port IS NOT NULL AND status IN ('pending', 'building', 'running')",
    )
    .fetch_all(&mut *conn)
    .await?;

    let allocated_ports: std::collections::HashSet<i32> =
        allocated.into_iter().map(|(p,)| p).collect();

    let port = (i32::from(range_start)..=i32::from(range_end))
        .find(|p| !allocated_ports.contains(p))
        .ok_or_else(|| AppError::Conflict("no available ports in configured range".to_string()))?;

    // Immediately claim the port in the same transaction
    let preview_url = format!("{base_url}:{port}");
    let now = Utc::now();
    sqlx::query(
        "UPDATE docker_previews SET port = $1, preview_url = $2, updated_at = $3 WHERE id = $4",
    )
    .bind(port)
    .bind(&preview_url)
    .bind(now)
    .bind(preview_id.to_string())
    .execute(&mut *conn)
    .await?;

    Ok(port)
}

/// Maximum concurrent Docker operations.
const MAX_CONCURRENT_DOCKER_OPS: usize = 3;

/// Manages Docker container lifecycle for preview environments.
pub struct PreviewManager {
    docker: Docker,
    pool: SqlitePool,
    sse_hub: Arc<SseHub>,
    config: Arc<Config>,
    repo_path: PathBuf,
    circuit_breaker: DockerCircuitBreaker,
    semaphore: Semaphore,
}

impl PreviewManager {
    pub fn new(
        config: Arc<Config>,
        pool: SqlitePool,
        sse_hub: Arc<SseHub>,
        repo_path: PathBuf,
    ) -> AppResult<Self> {
        let docker = if config.docker_host.starts_with("unix://") {
            Docker::connect_with_unix(&config.docker_host, 120, bollard::API_DEFAULT_VERSION)
                .map_err(|e| AppError::Internal(format!("docker connect error: {e}")))?
        } else {
            Docker::connect_with_http(&config.docker_host, 120, bollard::API_DEFAULT_VERSION)
                .map_err(|e| AppError::Internal(format!("docker connect error: {e}")))?
        };

        Ok(Self {
            docker,
            pool,
            sse_hub,
            config,
            repo_path,
            circuit_breaker: DockerCircuitBreaker::new(),
            semaphore: Semaphore::new(MAX_CONCURRENT_DOCKER_OPS),
        })
    }

    /// Build and start a Docker container for a preview.
    /// This runs as a background task; callers should `tokio::spawn` it.
    /// Check Docker circuit breaker state. Returns `true` if Docker is healthy.
    pub fn is_docker_healthy(&self) -> bool {
        self.circuit_breaker.state() != CircuitState::Open
    }

    #[tracing::instrument(skip(self), fields(%preview_id))]
    pub async fn build_and_start(&self, preview_id: Uuid) -> AppResult<()> {
        // Check circuit breaker before starting
        self.circuit_breaker.check()?;

        // Limit concurrent Docker operations
        let _permit = self
            .semaphore
            .acquire()
            .await
            .map_err(|_| AppError::Internal("Docker semaphore closed".to_string()))?;

        // 1. Get preview record
        let preview = get_preview(&self.pool, preview_id).await?;

        // 2. Update status to Building
        {
            let mut conn = self.pool.acquire().await?;
            let updated =
                update_status_tx(&mut conn, preview_id, PreviewStatus::Building, None).await?;
            self.sse_hub
                .broadcast(SseEvent::preview_status_changed(updated));
        }

        // 3. Build Docker image
        let worktree_path = self
            .repo_path
            .parent()
            .ok_or_else(|| AppError::Internal("repo has no parent dir".to_string()))?
            .join(&preview.worktree_name);

        let image_tag = format!("gantry-preview-{}", preview.worktree_name);

        match self.build_image(&worktree_path, &image_tag).await {
            Ok(()) => {
                self.circuit_breaker.record_success();
            }
            Err(e) => {
                self.circuit_breaker.record_failure();
                let mut conn = self.pool.acquire().await?;
                let updated = update_status_tx(
                    &mut conn,
                    preview_id,
                    PreviewStatus::Failed,
                    Some(format!("build failed: {e}")),
                )
                .await?;
                self.sse_hub
                    .broadcast(SseEvent::preview_status_changed(updated));
                return Err(e);
            }
        }

        // 4. Allocate port atomically via BEGIN IMMEDIATE transaction.
        // sqlx 0.8 defaults to BEGIN DEFERRED for SQLite, which only acquires
        // a write lock on the first write statement. Using BEGIN IMMEDIATE
        // acquires the write lock immediately, preventing SQLITE_BUSY errors
        // when concurrent allocations race between BEGIN and the first write.
        let port = {
            let mut conn = self.pool.acquire().await?;
            sqlx::query("BEGIN IMMEDIATE").execute(&mut *conn).await?;
            let port = match allocate_port_tx(
                &mut conn,
                preview_id,
                self.config.preview_port_range_start,
                self.config.preview_port_range_end,
                &self.config.preview_base_url,
            )
            .await
            {
                Ok(port) => port,
                Err(e) => {
                    let _ = sqlx::query("ROLLBACK").execute(&mut *conn).await;
                    return Err(e);
                }
            };
            sqlx::query("COMMIT").execute(&mut *conn).await?;
            port
        };

        // 5. Create and start container
        let container_name = format!("gantry-preview-{}", preview.worktree_name);

        match self
            .create_and_start_container(&image_tag, &container_name, port)
            .await
        {
            Ok(container_id) => {
                self.circuit_breaker.record_success();
                let mut conn = self.pool.acquire().await?;
                update_container_id_tx(&mut conn, preview_id, &container_id).await?;
                let updated =
                    update_status_tx(&mut conn, preview_id, PreviewStatus::Running, None).await?;
                self.sse_hub
                    .broadcast(SseEvent::preview_status_changed(updated));
            }
            Err(e) => {
                self.circuit_breaker.record_failure();
                let mut conn = self.pool.acquire().await?;
                let updated = update_status_tx(
                    &mut conn,
                    preview_id,
                    PreviewStatus::Failed,
                    Some(format!("start failed: {e}")),
                )
                .await?;
                self.sse_hub
                    .broadcast(SseEvent::preview_status_changed(updated));
                return Err(e);
            }
        }

        Ok(())
    }

    /// Stop a running container.
    #[tracing::instrument(skip(self), fields(%preview_id))]
    pub async fn stop(&self, preview_id: Uuid) -> AppResult<DockerPreview> {
        let preview = get_preview(&self.pool, preview_id).await?;

        #[allow(deprecated)]
        if let Some(ref container_id) = preview.container_id {
            let _ = self
                .docker
                .stop_container(container_id, Some(StopContainerOptions { t: 10 }))
                .await;
        }

        let mut conn = self.pool.acquire().await?;
        let updated = update_status_tx(&mut conn, preview_id, PreviewStatus::Stopped, None).await?;
        self.sse_hub
            .broadcast(SseEvent::preview_status_changed(updated.clone()));
        Ok(updated)
    }

    /// Stop and remove the Docker container for a preview.
    #[tracing::instrument(skip(self), fields(%preview_id))]
    pub async fn cleanup(&self, preview_id: Uuid) -> AppResult<()> {
        let preview = get_preview(&self.pool, preview_id).await?;

        #[allow(deprecated)]
        if let Some(ref container_id) = preview.container_id {
            let _ = self
                .docker
                .stop_container(container_id, Some(StopContainerOptions { t: 5 }))
                .await;
            let _ = self
                .docker
                .remove_container(
                    container_id,
                    Some(RemoveContainerOptions {
                        force: true,
                        ..Default::default()
                    }),
                )
                .await;
        }

        Ok(())
    }

    #[allow(deprecated)]
    async fn build_image(&self, worktree_path: &std::path::Path, tag: &str) -> AppResult<()> {
        // Check Dockerfile exists
        let dockerfile_path = worktree_path.join("Dockerfile");
        if !dockerfile_path.exists() {
            return Err(AppError::NotFound(format!(
                "no Dockerfile found in worktree at {}",
                worktree_path.display()
            )));
        }

        // Create tar archive of the build context
        let tar_bytes = create_tar_context(worktree_path)?;

        let options = BuildImageOptions {
            t: tag.to_string(),
            dockerfile: "Dockerfile".to_string(),
            rm: true,
            ..Default::default()
        };

        let body = http_body_util::Either::Left(Full::new(Bytes::from(tar_bytes)));
        let mut stream = self.docker.build_image(options, None, Some(body));

        while let Some(result) = stream.next().await {
            match result {
                Ok(output) => {
                    if let Some(err) = output.error {
                        return Err(AppError::Internal(format!("docker build error: {err}")));
                    }
                    if let Some(stream_msg) = output.stream {
                        info!(target: "preview_build", "{}", stream_msg.trim());
                    }
                }
                Err(e) => {
                    return Err(AppError::Internal(format!("docker build error: {e}")));
                }
            }
        }

        Ok(())
    }

    #[allow(deprecated)]
    async fn create_and_start_container(
        &self,
        image: &str,
        name: &str,
        host_port: i32,
    ) -> AppResult<String> {
        // Remove old container with same name if exists
        #[allow(deprecated)]
        let _ = self
            .docker
            .remove_container(
                name,
                Some(RemoveContainerOptions {
                    force: true,
                    ..Default::default()
                }),
            )
            .await;

        let mut port_bindings = PortMap::new();
        port_bindings.insert(
            "8080/tcp".to_string(),
            Some(vec![PortBinding {
                host_ip: Some("127.0.0.1".to_string()),
                host_port: Some(host_port.to_string()),
            }]),
        );

        let host_config = HostConfig {
            port_bindings: Some(port_bindings),
            memory: Some(512 * 1024 * 1024), // 512 MB memory limit
            memory_swap: Some(512 * 1024 * 1024), // No swap (same as memory)
            pids_limit: Some(256),           // Max 256 processes
            readonly_rootfs: Some(true),     // Read-only root filesystem
            security_opt: Some(vec!["no-new-privileges:true".to_string()]),
            tmpfs: Some(HashMap::from([
                ("/tmp".to_string(), "rw,noexec,nosuid,size=64m".to_string()),
                (
                    "/var/log".to_string(),
                    "rw,noexec,nosuid,size=32m".to_string(),
                ),
                ("/run".to_string(), "rw,noexec,nosuid,size=16m".to_string()),
            ])),
            ..Default::default()
        };

        let container_config = ContainerConfig {
            image: Some(image.to_string()),
            host_config: Some(host_config),
            ..Default::default()
        };

        let create_options = CreateContainerOptions {
            name: name.to_string(),
            ..Default::default()
        };

        let container = self
            .docker
            .create_container(Some(create_options), container_config)
            .await
            .map_err(|e| AppError::Internal(format!("create container error: {e}")))?;

        #[allow(deprecated)]
        self.docker
            .start_container(&container.id, None::<StartContainerOptions<String>>)
            .await
            .map_err(|e| AppError::Internal(format!("start container error: {e}")))?;

        Ok(container.id)
    }
}

fn create_tar_context(dir: &std::path::Path) -> AppResult<Vec<u8>> {
    use std::io::Write;

    let buf = Vec::new();
    let mut archive = tar::Builder::new(buf);
    archive
        .append_dir_all(".", dir)
        .map_err(|e| AppError::Internal(format!("tar creation error: {e}")))?;
    let mut buf = archive
        .into_inner()
        .map_err(|e| AppError::Internal(format!("tar finalize error: {e}")))?;
    buf.flush()
        .map_err(|e| AppError::Internal(format!("tar flush error: {e}")))?;
    Ok(buf)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::test_helpers::setup_test_db;

    async fn insert_preview(pool: &SqlitePool, worktree_name: &str) -> Uuid {
        let id = Uuid::new_v4();
        let now = Utc::now();
        sqlx::query(
            "INSERT INTO docker_previews (id, worktree_name, status, created_at, updated_at) VALUES ($1, $2, 'pending', $3, $4)",
        )
        .bind(id.to_string())
        .bind(worktree_name)
        .bind(now)
        .bind(now)
        .execute(pool)
        .await
        .expect("insert preview");
        id
    }

    #[tokio::test]
    async fn test_allocate_port_assigns_first_available() {
        let pool = setup_test_db().await;
        let id = insert_preview(&pool, "wt-1").await;

        let mut conn = pool.acquire().await.unwrap();
        let port = allocate_port_tx(&mut conn, id, 9000, 9010, "http://localhost")
            .await
            .expect("port allocation should succeed");

        assert_eq!(port, 9000);
    }

    #[tokio::test]
    async fn test_allocate_port_skips_already_allocated() {
        let pool = setup_test_db().await;
        let id1 = insert_preview(&pool, "wt-1").await;
        let id2 = insert_preview(&pool, "wt-2").await;

        let mut conn = pool.acquire().await.unwrap();
        let port1 = allocate_port_tx(&mut conn, id1, 9000, 9010, "http://localhost")
            .await
            .expect("first allocation should succeed");
        assert_eq!(port1, 9000);

        let port2 = allocate_port_tx(&mut conn, id2, 9000, 9010, "http://localhost")
            .await
            .expect("second allocation should succeed");
        assert_eq!(port2, 9001);
    }

    #[tokio::test]
    async fn test_port_unique_constraint_prevents_duplicate() {
        let pool = setup_test_db().await;
        let id1 = insert_preview(&pool, "wt-1").await;
        let id2 = insert_preview(&pool, "wt-2").await;

        // Allocate port 9000 to id1
        let mut conn = pool.acquire().await.unwrap();
        allocate_port_tx(&mut conn, id1, 9000, 9010, "http://localhost")
            .await
            .expect("first allocation should succeed");

        // Directly try to set id2's port to 9000 — should fail due to UNIQUE index
        let result = sqlx::query("UPDATE docker_previews SET port = 9000 WHERE id = $1")
            .bind(id2.to_string())
            .execute(&pool)
            .await;

        assert!(
            result.is_err(),
            "duplicate port should be rejected by UNIQUE constraint"
        );
    }
}
