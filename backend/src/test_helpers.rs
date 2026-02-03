//! Shared test utilities for service and integration tests.

use sqlx::sqlite::SqlitePoolOptions;
use sqlx::SqlitePool;

/// Creates an in-memory SQLite database with migrations applied.
///
/// # Panics
/// Panics if database creation or migration fails.
pub async fn setup_test_db() -> SqlitePool {
    let pool = SqlitePoolOptions::new()
        .connect("sqlite::memory:")
        .await
        .expect("Failed to create test database");

    sqlx::migrate!("./migrations")
        .run(&pool)
        .await
        .expect("Failed to run migrations");

    pool
}
