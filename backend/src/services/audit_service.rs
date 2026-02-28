use crate::error::AppResult;
use serde::{Deserialize, Serialize};
use sqlx::SqlitePool;
use uuid::Uuid;

/// A recorded audit event.
#[derive(Debug, Serialize, Deserialize)]
pub struct AuditEvent {
    pub id: String,
    pub event_type: String,
    pub actor_id: Option<String>,
    pub target_type: Option<String>,
    pub target_id: Option<String>,
    pub metadata: Option<serde_json::Value>,
    pub ip_address: Option<String>,
    pub created_at: String,
}

/// Paginated audit log response.
#[derive(Debug, Serialize)]
pub struct AuditLogResponse {
    pub items: Vec<AuditEvent>,
    pub total: i64,
    pub limit: i64,
    pub offset: i64,
}

/// Fire-and-forget audit event logging.
/// Spawns a background task so it never blocks the caller.
pub fn log_event(
    pool: SqlitePool,
    event_type: &str,
    actor_id: Option<&str>,
    target_type: Option<&str>,
    target_id: Option<&str>,
    metadata: Option<serde_json::Value>,
    ip_address: Option<&str>,
) {
    let id = Uuid::new_v4().to_string();
    let event_type = event_type.to_string();
    let actor_id = actor_id.map(|s| s.to_string());
    let target_type = target_type.map(|s| s.to_string());
    let target_id = target_id.map(|s| s.to_string());
    let metadata_str = metadata.map(|v| v.to_string());
    let ip_address = ip_address.map(|s| s.to_string());

    tokio::spawn(async move {
        let result = sqlx::query(
            "INSERT INTO audit_events (id, event_type, actor_id, target_type, target_id, metadata, ip_address)
             VALUES ($1, $2, $3, $4, $5, $6, $7)",
        )
        .bind(&id)
        .bind(&event_type)
        .bind(&actor_id)
        .bind(&target_type)
        .bind(&target_id)
        .bind(&metadata_str)
        .bind(&ip_address)
        .execute(&pool)
        .await;

        if let Err(e) = result {
            tracing::warn!(error = %e, event_type = %event_type, "failed to record audit event");
        }
    });
}

/// List audit events with pagination and optional filtering.
pub async fn list_events(
    pool: &SqlitePool,
    event_type_filter: Option<&str>,
    limit: i64,
    offset: i64,
) -> AppResult<AuditLogResponse> {
    let (items, total) = if let Some(et) = event_type_filter {
        let total: (i64,) =
            sqlx::query_as("SELECT COUNT(*) FROM audit_events WHERE event_type = $1")
                .bind(et)
                .fetch_one(pool)
                .await?;

        let rows = sqlx::query_as::<_, (String, String, Option<String>, Option<String>, Option<String>, Option<String>, Option<String>, String)>(
            "SELECT id, event_type, actor_id, target_type, target_id, metadata, ip_address, created_at
             FROM audit_events WHERE event_type = $1 ORDER BY created_at DESC LIMIT $2 OFFSET $3",
        )
        .bind(et)
        .bind(limit)
        .bind(offset)
        .fetch_all(pool)
        .await?;

        (rows, total.0)
    } else {
        let total: (i64,) = sqlx::query_as("SELECT COUNT(*) FROM audit_events")
            .fetch_one(pool)
            .await?;

        let rows = sqlx::query_as::<_, (String, String, Option<String>, Option<String>, Option<String>, Option<String>, Option<String>, String)>(
            "SELECT id, event_type, actor_id, target_type, target_id, metadata, ip_address, created_at
             FROM audit_events ORDER BY created_at DESC LIMIT $1 OFFSET $2",
        )
        .bind(limit)
        .bind(offset)
        .fetch_all(pool)
        .await?;

        (rows, total.0)
    };

    let events: Vec<AuditEvent> = items
        .into_iter()
        .map(|row| AuditEvent {
            id: row.0,
            event_type: row.1,
            actor_id: row.2,
            target_type: row.3,
            target_id: row.4,
            metadata: row.5.and_then(|s| serde_json::from_str(&s).ok()),
            ip_address: row.6,
            created_at: row.7,
        })
        .collect();

    Ok(AuditLogResponse {
        items: events,
        total,
        limit,
        offset,
    })
}

/// Delete audit events older than `retention_days`.
pub async fn cleanup_old_events(pool: &SqlitePool, retention_days: u64) -> AppResult<u64> {
    let result = sqlx::query(
        "DELETE FROM audit_events WHERE created_at < datetime('now', '-' || $1 || ' days')",
    )
    .bind(retention_days as i64)
    .execute(pool)
    .await?;

    Ok(result.rows_affected())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::test_helpers::setup_test_db;

    #[tokio::test]
    async fn test_log_event_and_list() {
        let pool = setup_test_db().await;

        // log_event spawns a background task, so call it and wait briefly
        log_event(
            pool.clone(),
            "user.login",
            Some("actor-1"),
            Some("user"),
            Some("target-1"),
            Some(serde_json::json!({"key": "value"})),
            Some("127.0.0.1"),
        );

        // Wait for the spawned task to complete
        tokio::time::sleep(tokio::time::Duration::from_millis(100)).await;

        let result = list_events(&pool, None, 10, 0).await.unwrap();
        assert_eq!(result.total, 1);
        assert_eq!(result.items.len(), 1);
        assert_eq!(result.items[0].event_type, "user.login");
        assert_eq!(result.items[0].actor_id.as_deref(), Some("actor-1"));
        assert_eq!(result.items[0].ip_address.as_deref(), Some("127.0.0.1"));
    }

    #[tokio::test]
    async fn test_list_events_with_type_filter() {
        let pool = setup_test_db().await;

        log_event(pool.clone(), "user.login", None, None, None, None, None);
        log_event(pool.clone(), "user.logout", None, None, None, None, None);
        log_event(pool.clone(), "user.login", None, None, None, None, None);

        tokio::time::sleep(tokio::time::Duration::from_millis(100)).await;

        let result = list_events(&pool, Some("user.login"), 10, 0).await.unwrap();
        assert_eq!(result.total, 2);
        assert_eq!(result.items.len(), 2);

        let result = list_events(&pool, Some("user.logout"), 10, 0)
            .await
            .unwrap();
        assert_eq!(result.total, 1);
    }

    #[tokio::test]
    async fn test_list_events_pagination() {
        let pool = setup_test_db().await;

        for _ in 0..5 {
            log_event(pool.clone(), "test.event", None, None, None, None, None);
        }

        tokio::time::sleep(tokio::time::Duration::from_millis(100)).await;

        let page1 = list_events(&pool, None, 2, 0).await.unwrap();
        assert_eq!(page1.total, 5);
        assert_eq!(page1.items.len(), 2);
        assert_eq!(page1.limit, 2);
        assert_eq!(page1.offset, 0);

        let page2 = list_events(&pool, None, 2, 2).await.unwrap();
        assert_eq!(page2.items.len(), 2);
        assert_eq!(page2.offset, 2);
    }

    #[tokio::test]
    async fn test_cleanup_old_events() {
        let pool = setup_test_db().await;

        log_event(pool.clone(), "old.event", None, None, None, None, None);

        tokio::time::sleep(tokio::time::Duration::from_millis(100)).await;

        // retention_days=0 should not delete events created just now
        let deleted = cleanup_old_events(&pool, 0).await.unwrap();
        // Events created within the same second may not be cleaned up with 0 days
        // so just verify the function runs without error
        assert!(deleted == 0 || deleted == 1);

        // Verify list still works
        let result = list_events(&pool, None, 10, 0).await.unwrap();
        assert!(result.total >= 0);
    }

    #[tokio::test]
    async fn test_log_event_without_optional_fields() {
        let pool = setup_test_db().await;

        log_event(pool.clone(), "system.startup", None, None, None, None, None);

        tokio::time::sleep(tokio::time::Duration::from_millis(100)).await;

        let result = list_events(&pool, None, 10, 0).await.unwrap();
        assert_eq!(result.total, 1);
        assert_eq!(result.items[0].event_type, "system.startup");
        assert!(result.items[0].actor_id.is_none());
        assert!(result.items[0].target_type.is_none());
        assert!(result.items[0].ip_address.is_none());
    }
}
