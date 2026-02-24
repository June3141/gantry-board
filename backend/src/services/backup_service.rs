use std::path::{Path, PathBuf};
use std::time::Duration;

use sqlx::SqlitePool;
use tokio_util::sync::CancellationToken;

use crate::error::AppResult;

/// Create a SQLite backup using VACUUM INTO.
///
/// Steps:
/// 1. Execute `PRAGMA wal_checkpoint(TRUNCATE)` to flush the WAL
/// 2. Execute `VACUUM INTO '<backup_dir>/gantry_board_YYYYMMDD_HHMMSS.db'`
/// 3. Return the path to the created backup file
pub async fn create_backup(pool: &SqlitePool, backup_dir: &Path) -> AppResult<PathBuf> {
    let start = std::time::Instant::now();

    // Ensure backup directory exists
    std::fs::create_dir_all(backup_dir).map_err(|e| {
        crate::error::AppError::Internal(format!(
            "failed to create backup directory {}: {}",
            backup_dir.display(),
            e
        ))
    })?;

    // Flush WAL to database file
    sqlx::query("PRAGMA wal_checkpoint(TRUNCATE)")
        .execute(pool)
        .await?;

    // Generate backup filename with timestamp
    let now = chrono::Utc::now();
    let filename = format!("gantry_board_{}.db", now.format("%Y%m%d_%H%M%S"));
    let backup_path = backup_dir.join(&filename);

    // VACUUM INTO creates an independent copy of the database
    let path_str = backup_path
        .to_str()
        .ok_or_else(|| {
            crate::error::AppError::Internal("backup path contains invalid UTF-8".to_string())
        })?
        .to_string();

    sqlx::query(&format!("VACUUM INTO '{}'", path_str.replace('\'', "''")))
        .execute(pool)
        .await?;

    let duration = start.elapsed();
    let file_size = std::fs::metadata(&backup_path)
        .map(|m| m.len())
        .unwrap_or(0);

    tracing::info!(
        path = %backup_path.display(),
        size_bytes = file_size,
        duration_ms = duration.as_millis() as u64,
        "SQLite backup created"
    );

    Ok(backup_path)
}

/// Rotate backup files, keeping only `retention_count` most recent.
///
/// Lists backup files matching the naming pattern `gantry_board_*.db`,
/// sorts by name (which includes timestamps), and deletes the oldest
/// files beyond the retention count. Returns the number of files deleted.
pub fn rotate_backups(backup_dir: &Path, retention_count: u32) -> AppResult<u32> {
    let entries = match std::fs::read_dir(backup_dir) {
        Ok(entries) => entries,
        Err(e) if e.kind() == std::io::ErrorKind::NotFound => return Ok(0),
        Err(e) => {
            return Err(crate::error::AppError::Internal(format!(
                "failed to read backup directory {}: {}",
                backup_dir.display(),
                e
            )));
        }
    };

    // Collect backup files matching our naming pattern
    let mut backup_files: Vec<String> = entries
        .filter_map(|e| e.ok())
        .filter_map(|e| {
            let name = e.file_name().to_string_lossy().to_string();
            if name.starts_with("gantry_board_") && name.ends_with(".db") {
                Some(name)
            } else {
                None
            }
        })
        .collect();

    // Sort by name (timestamp order, oldest first)
    backup_files.sort();

    let total = backup_files.len();
    let to_delete = total.saturating_sub(retention_count as usize);

    if to_delete == 0 {
        return Ok(0);
    }

    let mut deleted = 0u32;
    for name in backup_files.iter().take(to_delete) {
        let path = backup_dir.join(name);
        if let Err(e) = std::fs::remove_file(&path) {
            tracing::warn!(
                path = %path.display(),
                error = %e,
                "failed to delete old backup file"
            );
        } else {
            deleted += 1;
        }
    }

    if deleted > 0 {
        tracing::info!(deleted, "rotated old backup files");
    }

    Ok(deleted)
}

/// Spawn a background task that periodically creates backups and rotates old ones.
///
/// Uses the CancellationToken pattern for graceful shutdown.
pub fn spawn_backup_task(
    pool: SqlitePool,
    config: &crate::config::Config,
    cancel_token: CancellationToken,
) {
    let interval_secs = config.backup_interval_secs.max(60);
    let backup_dir = PathBuf::from(&config.backup_dir);
    let retention_count = config.backup_retention_count;

    tracing::info!(
        interval_secs,
        backup_dir = %backup_dir.display(),
        retention_count,
        "background backup task started"
    );

    tokio::spawn(async move {
        let mut interval = tokio::time::interval(Duration::from_secs(interval_secs));
        interval.set_missed_tick_behavior(tokio::time::MissedTickBehavior::Delay);
        interval.tick().await; // skip the immediate first tick

        loop {
            tokio::select! {
                _ = interval.tick() => {}
                _ = cancel_token.cancelled() => {
                    tracing::info!("backup task shutting down");
                    return;
                }
            }

            match create_backup(&pool, &backup_dir).await {
                Ok(_) => {
                    if let Err(e) = rotate_backups(&backup_dir, retention_count) {
                        tracing::warn!(error = %e, "backup rotation failed");
                    }
                }
                Err(e) => {
                    tracing::error!(error = %e, "scheduled backup failed");
                }
            }
        }
    });
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::test_helpers::setup_test_db;
    use std::fs;
    use tempfile::TempDir;

    #[tokio::test]
    async fn test_create_backup_creates_valid_sqlite_file() {
        let pool = setup_test_db().await;
        let backup_dir = TempDir::new().expect("create temp dir");

        let backup_path = create_backup(&pool, backup_dir.path())
            .await
            .expect("backup should succeed");

        // File should exist
        assert!(backup_path.exists(), "backup file should exist");

        // File should be non-empty
        let metadata = fs::metadata(&backup_path).expect("metadata");
        assert!(metadata.len() > 0, "backup file should be non-empty");

        // File name should match pattern gantry_board_YYYYMMDD_HHMMSS.db
        let file_name = backup_path
            .file_name()
            .expect("file name")
            .to_str()
            .expect("utf8");
        assert!(
            file_name.starts_with("gantry_board_"),
            "file name should start with gantry_board_"
        );
        assert!(file_name.ends_with(".db"), "file name should end with .db");

        // Should be a valid SQLite database (try to open it)
        let backup_url = format!("sqlite:{}?mode=ro", backup_path.display());
        let backup_pool = sqlx::sqlite::SqlitePoolOptions::new()
            .connect(&backup_url)
            .await
            .expect("should be valid SQLite");
        backup_pool.close().await;
    }

    #[tokio::test]
    async fn test_create_backup_creates_backup_dir_if_missing() {
        let pool = setup_test_db().await;
        let base_dir = TempDir::new().expect("create temp dir");
        let backup_dir = base_dir.path().join("nested").join("backups");

        let backup_path = create_backup(&pool, &backup_dir)
            .await
            .expect("backup should succeed");

        assert!(backup_path.exists(), "backup file should exist");
        assert!(backup_dir.exists(), "backup dir should have been created");
    }

    #[test]
    fn test_rotate_backups_keeps_retention_count() {
        let backup_dir = TempDir::new().expect("create temp dir");

        // Create 5 fake backup files with sequential timestamps
        let names = [
            "gantry_board_20260101_000000.db",
            "gantry_board_20260102_000000.db",
            "gantry_board_20260103_000000.db",
            "gantry_board_20260104_000000.db",
            "gantry_board_20260105_000000.db",
        ];
        for name in &names {
            fs::write(backup_dir.path().join(name), b"dummy").expect("write");
        }

        // Keep only 3
        let deleted = rotate_backups(backup_dir.path(), 3).expect("rotate should succeed");
        assert_eq!(deleted, 2, "should delete 2 oldest backups");

        // Verify the 3 newest remain
        let remaining: Vec<String> = fs::read_dir(backup_dir.path())
            .expect("read dir")
            .filter_map(|e| e.ok())
            .map(|e| e.file_name().to_string_lossy().to_string())
            .collect();
        assert_eq!(remaining.len(), 3);
        assert!(!remaining.contains(&"gantry_board_20260101_000000.db".to_string()));
        assert!(!remaining.contains(&"gantry_board_20260102_000000.db".to_string()));
        assert!(remaining.contains(&"gantry_board_20260103_000000.db".to_string()));
        assert!(remaining.contains(&"gantry_board_20260104_000000.db".to_string()));
        assert!(remaining.contains(&"gantry_board_20260105_000000.db".to_string()));
    }

    #[test]
    fn test_rotate_backups_no_op_when_under_retention() {
        let backup_dir = TempDir::new().expect("create temp dir");

        // Create 2 files
        fs::write(
            backup_dir.path().join("gantry_board_20260101_000000.db"),
            b"dummy",
        )
        .expect("write");
        fs::write(
            backup_dir.path().join("gantry_board_20260102_000000.db"),
            b"dummy",
        )
        .expect("write");

        // Keep 5
        let deleted = rotate_backups(backup_dir.path(), 5).expect("rotate should succeed");
        assert_eq!(deleted, 0, "should delete nothing when under retention");
    }

    #[test]
    fn test_rotate_backups_ignores_non_backup_files() {
        let backup_dir = TempDir::new().expect("create temp dir");

        // Create backup files and some non-backup files
        fs::write(
            backup_dir.path().join("gantry_board_20260101_000000.db"),
            b"dummy",
        )
        .expect("write");
        fs::write(
            backup_dir.path().join("gantry_board_20260102_000000.db"),
            b"dummy",
        )
        .expect("write");
        fs::write(backup_dir.path().join("other_file.txt"), b"dummy").expect("write");
        fs::write(backup_dir.path().join("README.md"), b"dummy").expect("write");

        // Keep 1
        let deleted = rotate_backups(backup_dir.path(), 1).expect("rotate should succeed");
        assert_eq!(deleted, 1, "should only delete 1 backup file");

        // Non-backup files should still exist
        assert!(backup_dir.path().join("other_file.txt").exists());
        assert!(backup_dir.path().join("README.md").exists());
    }

    #[test]
    fn test_rotate_backups_handles_empty_dir() {
        let backup_dir = TempDir::new().expect("create temp dir");

        let deleted = rotate_backups(backup_dir.path(), 3).expect("rotate should succeed");
        assert_eq!(deleted, 0);
    }
}
