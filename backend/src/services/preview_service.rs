use std::collections::HashMap;
use std::path::PathBuf;
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
use tracing::info;
use uuid::Uuid;

use crate::config::Config;
use crate::error::{AppError, AppResult};
use crate::models::docker_preview::{DockerPreview, PreviewStatus};
use crate::sse::event::SseEvent;
use crate::sse::hub::SseHub;

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

/// Manages Docker container lifecycle for preview environments.
pub struct PreviewManager {
    docker: Docker,
    pool: SqlitePool,
    sse_hub: Arc<SseHub>,
    config: Arc<Config>,
    repo_path: PathBuf,
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
        })
    }

    /// Build and start a Docker container for a preview.
    /// This runs as a background task; callers should `tokio::spawn` it.
    #[tracing::instrument(skip(self), fields(%preview_id))]
    pub async fn build_and_start(&self, preview_id: Uuid) -> AppResult<()> {
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
            Ok(()) => {}
            Err(e) => {
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

        // 4. Allocate port
        let port = find_available_port(
            &self.pool,
            self.config.preview_port_range_start,
            self.config.preview_port_range_end,
        )
        .await?;

        // 5. Create and start container
        let container_name = format!("gantry-preview-{}", preview.worktree_name);
        let preview_url = format!("{}:{}", self.config.preview_base_url, port);

        match self
            .create_and_start_container(&image_tag, &container_name, port)
            .await
        {
            Ok(container_id) => {
                let mut conn = self.pool.acquire().await?;
                update_container_info_tx(&mut conn, preview_id, &container_id, port, &preview_url)
                    .await?;
                let updated =
                    update_status_tx(&mut conn, preview_id, PreviewStatus::Running, None).await?;
                self.sse_hub
                    .broadcast(SseEvent::preview_status_changed(updated));
            }
            Err(e) => {
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
