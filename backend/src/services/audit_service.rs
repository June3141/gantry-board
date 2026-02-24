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
             VALUES (?, ?, ?, ?, ?, ?, ?)",
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
) -> Result<AuditLogResponse, sqlx::Error> {
    let (items, total) = if let Some(et) = event_type_filter {
        let total: (i64,) =
            sqlx::query_as("SELECT COUNT(*) FROM audit_events WHERE event_type = ?")
                .bind(et)
                .fetch_one(pool)
                .await?;

        let rows = sqlx::query_as::<_, (String, String, Option<String>, Option<String>, Option<String>, Option<String>, Option<String>, String)>(
            "SELECT id, event_type, actor_id, target_type, target_id, metadata, ip_address, created_at
             FROM audit_events WHERE event_type = ? ORDER BY created_at DESC LIMIT ? OFFSET ?",
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
             FROM audit_events ORDER BY created_at DESC LIMIT ? OFFSET ?",
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
pub async fn cleanup_old_events(
    pool: &SqlitePool,
    retention_days: u64,
) -> Result<u64, sqlx::Error> {
    let result = sqlx::query(
        "DELETE FROM audit_events WHERE created_at < datetime('now', '-' || ? || ' days')",
    )
    .bind(retention_days as i64)
    .execute(pool)
    .await?;

    Ok(result.rows_affected())
}
