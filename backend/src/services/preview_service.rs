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
use futures_util::StreamExt;
use http_body_util::Full;
use tokio::sync::Semaphore;
use tracing::info;
use uuid::Uuid;

use crate::config::Config;
use crate::error::{AppError, AppResult};
use crate::models::docker_preview::{DockerPreview, PreviewStatus};
use crate::realtime::event::SseEvent;
use crate::realtime::hub::SseHub;
use crate::services::preview_repository::{
    allocate_port_tx, get_preview, update_container_id_tx, update_status_tx,
};

/// Circuit breaker for Docker operations.
/// Opens after `FAILURE_THRESHOLD` consecutive failures and stays open
/// for `RECOVERY_WINDOW` seconds before allowing a single probe request.
struct DockerCircuitBreaker {
    consecutive_failures: AtomicU32,
    last_failure_epoch: std::sync::atomic::AtomicU64,
}

const FAILURE_THRESHOLD: u32 = 3;
const RECOVERY_WINDOW_SECS: u64 = 30;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum CircuitState {
    Closed,
    Open,
    HalfOpen,
}

impl DockerCircuitBreaker {
    fn new() -> Self {
        Self {
            consecutive_failures: AtomicU32::new(0),
            last_failure_epoch: std::sync::atomic::AtomicU64::new(0),
        }
    }

    fn state(&self) -> CircuitState {
        let failures = self.consecutive_failures.load(Ordering::Acquire);
        if failures < FAILURE_THRESHOLD {
            return CircuitState::Closed;
        }
        let last = self.last_failure_epoch.load(Ordering::Acquire);
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

    fn check(&self) -> AppResult<()> {
        match self.state() {
            CircuitState::Closed | CircuitState::HalfOpen => Ok(()),
            CircuitState::Open => Err(AppError::Internal(
                "Docker operations temporarily unavailable (circuit breaker open)".to_string(),
            )),
        }
    }

    fn record_success(&self) {
        self.consecutive_failures.store(0, Ordering::Release);
    }

    fn record_failure(&self) {
        self.consecutive_failures.fetch_add(1, Ordering::AcqRel);
        let now = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap_or_default()
            .as_secs();
        self.last_failure_epoch.store(now, Ordering::Release);
    }
}

impl Default for DockerCircuitBreaker {
    fn default() -> Self {
        Self::new()
    }
}

/// Maximum concurrent Docker operations.
const MAX_CONCURRENT_DOCKER_OPS: usize = 3;

/// Manages Docker container lifecycle for preview environments.
pub struct PreviewManager {
    docker: Docker,
    pool: sqlx::SqlitePool,
    sse_hub: Arc<SseHub>,
    config: Arc<Config>,
    repo_path: PathBuf,
    circuit_breaker: DockerCircuitBreaker,
    semaphore: Semaphore,
}

impl PreviewManager {
    pub fn new(
        config: Arc<Config>,
        pool: sqlx::SqlitePool,
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

    /// Check Docker circuit breaker state. Returns `true` if Docker is healthy.
    pub fn is_docker_healthy(&self) -> bool {
        self.circuit_breaker.state() != CircuitState::Open
    }

    /// Build and start a Docker container for a preview.
    /// This runs as a background task; callers should `tokio::spawn` it.
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
