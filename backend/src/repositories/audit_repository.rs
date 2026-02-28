use sqlx::SqlitePool;

use crate::error::AppResult;

/// Parameters for inserting a new audit event.
pub struct InsertEventParams<'a> {
    pub id: &'a str,
    pub event_type: &'a str,
    pub actor_id: Option<&'a str>,
    pub target_type: Option<&'a str>,
    pub target_id: Option<&'a str>,
    pub metadata: Option<&'a str>,
    pub ip_address: Option<&'a str>,
}

/// Insert a raw audit event row.
pub async fn insert_event(pool: &SqlitePool, params: &InsertEventParams<'_>) -> AppResult<()> {
    sqlx::query(
        "INSERT INTO audit_events (id, event_type, actor_id, target_type, target_id, metadata, ip_address)
         VALUES ($1, $2, $3, $4, $5, $6, $7)",
    )
    .bind(params.id)
    .bind(params.event_type)
    .bind(params.actor_id)
    .bind(params.target_type)
    .bind(params.target_id)
    .bind(params.metadata)
    .bind(params.ip_address)
    .execute(pool)
    .await?;

    Ok(())
}

/// Raw audit event row from the database.
pub type AuditEventRow = (
    String,
    String,
    Option<String>,
    Option<String>,
    Option<String>,
    Option<String>,
    Option<String>,
    String,
);

/// Count audit events, optionally filtered by event_type.
pub async fn count_events(pool: &SqlitePool, event_type_filter: Option<&str>) -> AppResult<i64> {
    let total: (i64,) = if let Some(et) = event_type_filter {
        sqlx::query_as("SELECT COUNT(*) FROM audit_events WHERE event_type = $1")
            .bind(et)
            .fetch_one(pool)
            .await?
    } else {
        sqlx::query_as("SELECT COUNT(*) FROM audit_events")
            .fetch_one(pool)
            .await?
    };
    Ok(total.0)
}

/// Fetch paginated audit events, optionally filtered by event_type.
pub async fn fetch_events(
    pool: &SqlitePool,
    event_type_filter: Option<&str>,
    limit: i64,
    offset: i64,
) -> AppResult<Vec<AuditEventRow>> {
    let rows = if let Some(et) = event_type_filter {
        sqlx::query_as::<_, AuditEventRow>(
            "SELECT id, event_type, actor_id, target_type, target_id, metadata, ip_address, created_at
             FROM audit_events WHERE event_type = $1 ORDER BY created_at DESC LIMIT $2 OFFSET $3",
        )
        .bind(et)
        .bind(limit)
        .bind(offset)
        .fetch_all(pool)
        .await?
    } else {
        sqlx::query_as::<_, AuditEventRow>(
            "SELECT id, event_type, actor_id, target_type, target_id, metadata, ip_address, created_at
             FROM audit_events ORDER BY created_at DESC LIMIT $1 OFFSET $2",
        )
        .bind(limit)
        .bind(offset)
        .fetch_all(pool)
        .await?
    };
    Ok(rows)
}

/// Delete audit events older than `retention_days`.
pub async fn delete_old_events(pool: &SqlitePool, retention_days: i64) -> AppResult<u64> {
    let result = sqlx::query(
        "DELETE FROM audit_events WHERE created_at < datetime('now', '-' || $1 || ' days')",
    )
    .bind(retention_days)
    .execute(pool)
    .await?;

    Ok(result.rows_affected())
}
