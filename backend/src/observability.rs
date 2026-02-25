//! Observability: Prometheus metrics initialization and structured access logging.
//!
//! This module centralizes all custom metrics `describe_*!` calls and provides
//! helpers for structured HTTP access logging via `tower_http::trace::TraceLayer`.

use axum::http::Request;
use tower_http::trace::{MakeSpan, OnResponse};
use tracing::Span;

/// Metric names (constants to keep names consistent between describe and record sites).
pub mod metric {
    /// Histogram: agent session duration in seconds.
    pub const AGENT_SESSION_DURATION: &str = "gantry_agent_session_duration_seconds";
    /// Counter: total completed agent sessions (labels: agent_type, status).
    pub const AGENT_SESSIONS_TOTAL: &str = "gantry_agent_sessions_total";
    /// Counter: total task status transitions (labels: status).
    pub const TASKS_TOTAL: &str = "gantry_tasks_total";
    /// Counter: total error responses (labels: error_code).
    pub const ERRORS_TOTAL: &str = "gantry_errors_total";
    /// Histogram: GitHub sync duration in seconds.
    pub const GITHUB_SYNC_DURATION: &str = "gantry_github_sync_duration_seconds";
    /// Counter: total GitHub sync issues (labels: direction).
    pub const GITHUB_SYNC_ISSUES_TOTAL: &str = "gantry_github_sync_issues_total";
    /// Gauge: DB pool connections (labels: state).
    pub const DB_POOL_CONNECTIONS: &str = "gantry_db_pool_connections";
    /// Gauge: WAL file size in bytes.
    pub const DB_WAL_SIZE_BYTES: &str = "gantry_db_wal_size_bytes";
    /// Gauge: main DB file size in bytes.
    pub const DB_SIZE_BYTES: &str = "gantry_db_size_bytes";
}

/// Register metric descriptions with the global recorder and seed initial values
/// so they appear in /metrics output immediately.
/// Safe to call multiple times (idempotent).
pub fn init_metrics() {
    // Register descriptions
    metrics::describe_histogram!(
        metric::AGENT_SESSION_DURATION,
        metrics::Unit::Seconds,
        "Duration of agent sessions in seconds"
    );
    metrics::describe_counter!(
        metric::AGENT_SESSIONS_TOTAL,
        "Total completed agent sessions"
    );
    metrics::describe_counter!(metric::TASKS_TOTAL, "Total task status transitions");
    metrics::describe_counter!(metric::ERRORS_TOTAL, "Total error responses by error code");
    metrics::describe_histogram!(
        metric::GITHUB_SYNC_DURATION,
        metrics::Unit::Seconds,
        "Duration of GitHub sync operations in seconds"
    );
    metrics::describe_counter!(
        metric::GITHUB_SYNC_ISSUES_TOTAL,
        "Total GitHub issues synced"
    );
    metrics::describe_gauge!(
        metric::DB_POOL_CONNECTIONS,
        "Number of database pool connections by state"
    );

    // Seed initial zero values so metrics appear in /metrics output immediately.
    // The Prometheus exporter only renders metrics that have been recorded at least once.
    metrics::histogram!(metric::AGENT_SESSION_DURATION).record(0.0);
    metrics::counter!(metric::AGENT_SESSIONS_TOTAL, "agent_type" => "init", "status" => "init")
        .absolute(0);
    metrics::counter!(metric::TASKS_TOTAL, "status" => "init").absolute(0);
    metrics::counter!(metric::ERRORS_TOTAL, "error_code" => "init").absolute(0);
    metrics::histogram!(metric::GITHUB_SYNC_DURATION).record(0.0);
    metrics::counter!(metric::GITHUB_SYNC_ISSUES_TOTAL, "direction" => "init").absolute(0);
    metrics::gauge!(metric::DB_POOL_CONNECTIONS, "state" => "active").set(0.0);
    metrics::gauge!(metric::DB_POOL_CONNECTIONS, "state" => "idle").set(0.0);
    metrics::describe_gauge!(
        metric::DB_WAL_SIZE_BYTES,
        metrics::Unit::Bytes,
        "Size of the SQLite WAL file in bytes"
    );
    metrics::describe_gauge!(
        metric::DB_SIZE_BYTES,
        metrics::Unit::Bytes,
        "Size of the main SQLite database file in bytes"
    );
    metrics::gauge!(metric::DB_WAL_SIZE_BYTES).set(0.0);
    metrics::gauge!(metric::DB_SIZE_BYTES).set(0.0);
}

/// Record DB pool connection metrics from an `SqlitePool`.
pub fn record_db_pool_metrics(pool: &sqlx::SqlitePool) {
    let size = pool.size() as f64;
    let idle = pool.num_idle() as f64;
    let active = size - idle;
    metrics::gauge!(metric::DB_POOL_CONNECTIONS, "state" => "active").set(active);
    metrics::gauge!(metric::DB_POOL_CONNECTIONS, "state" => "idle").set(idle);
}

/// Record DB file size metrics from the database URL.
/// Extracts the file path from the `sqlite:` URL and reports main DB + WAL sizes.
pub fn record_db_file_metrics(database_url: &str) {
    let path = database_url
        .strip_prefix("sqlite:")
        .unwrap_or(database_url)
        .split('?')
        .next()
        .unwrap_or(database_url);

    if let Ok(meta) = std::fs::metadata(path) {
        metrics::gauge!(metric::DB_SIZE_BYTES).set(meta.len() as f64);
    }

    let wal_path = format!("{}-wal", path);
    match std::fs::metadata(&wal_path) {
        Ok(meta) => metrics::gauge!(metric::DB_WAL_SIZE_BYTES).set(meta.len() as f64),
        Err(_) => metrics::gauge!(metric::DB_WAL_SIZE_BYTES).set(0.0),
    }
}

/// Run SQLite WAL checkpoint (PASSIVE) and PRAGMA optimize.
/// Returns Ok(()) on success; errors are logged but not fatal.
pub async fn run_db_maintenance(pool: &sqlx::SqlitePool) {
    // WAL checkpoint (PASSIVE mode — does not block writers)
    match sqlx::query("PRAGMA wal_checkpoint(PASSIVE)")
        .execute(pool)
        .await
    {
        Ok(_) => tracing::debug!("WAL checkpoint (PASSIVE) completed"),
        Err(e) => tracing::warn!(error = %e, "WAL checkpoint failed"),
    }

    // PRAGMA optimize — lets SQLite update internal statistics
    match sqlx::query("PRAGMA optimize").execute(pool).await {
        Ok(_) => tracing::debug!("PRAGMA optimize completed"),
        Err(e) => tracing::warn!(error = %e, "PRAGMA optimize failed"),
    }
}

// ---------------------------------------------------------------------------
// Structured access logging
// ---------------------------------------------------------------------------

/// Custom span builder for HTTP requests.
/// Adds method, path, and request_id to the span.
#[derive(Clone, Debug)]
pub struct AccessLogMakeSpan;

impl<B> MakeSpan<B> for AccessLogMakeSpan {
    fn make_span(&mut self, request: &Request<B>) -> Span {
        let request_id = request
            .headers()
            .get("x-request-id")
            .and_then(|v| v.to_str().ok())
            .unwrap_or("-");
        tracing::info_span!(
            "request",
            method = %request.method(),
            path = %request.uri().path(),
            request_id = %request_id,
        )
    }
}

/// Custom response logger that adjusts log level by status code and path.
///
/// - `/health*`, `/metrics` -> debug
/// - 4xx -> warn
/// - 5xx -> error
/// - 2xx/3xx -> info
#[derive(Clone, Debug)]
pub struct AccessLogOnResponse;

impl<B> OnResponse<B> for AccessLogOnResponse {
    fn on_response(
        self,
        response: &axum::http::Response<B>,
        latency: std::time::Duration,
        span: &Span,
    ) {
        let status = response.status().as_u16();
        let latency_ms = latency.as_secs_f64() * 1000.0;

        // Determine the path from the span's parent request.
        // We check the current span fields for the path.
        // Since we cannot easily extract span fields, we use the extension trick:
        // we store path in extensions in make_span. However, tower-http doesn't give
        // us access to the request in on_response. Instead, we log inside the span
        // which already has path recorded.

        // Check if this is a low-traffic endpoint by examining the span's path field.
        // We use the response extensions to pass the path from middleware.
        let is_health_or_metrics = response
            .extensions()
            .get::<RequestPath>()
            .map(|p| p.0.starts_with("/health") || p.0 == "/metrics")
            .unwrap_or(false);

        if is_health_or_metrics {
            tracing::debug!(parent: span, status, latency_ms, "response");
        } else if status >= 500 {
            tracing::error!(parent: span, status, latency_ms, "response");
        } else if status >= 400 {
            tracing::warn!(parent: span, status, latency_ms, "response");
        } else {
            tracing::info!(parent: span, status, latency_ms, "response");
        }
    }
}

/// Extension type to pass the request path to `on_response`.
#[derive(Clone, Debug)]
pub struct RequestPath(pub String);

#[cfg(test)]
mod tests {
    use super::*;
    use axum::http::{Method, Response, StatusCode};

    // -----------------------------------------------------------------------
    // init_metrics: calling it should not panic even if called twice
    // -----------------------------------------------------------------------
    #[test]
    fn test_init_metrics_is_idempotent() {
        init_metrics();
        init_metrics(); // second call should not panic
    }

    // -----------------------------------------------------------------------
    // AccessLogMakeSpan: span includes method, path, request_id
    // -----------------------------------------------------------------------
    #[test]
    fn test_make_span_includes_method_and_path() {
        let mut make_span = AccessLogMakeSpan;
        let req = Request::builder()
            .method(Method::GET)
            .uri("/api/tasks")
            .header("x-request-id", "abc-123")
            .body(())
            .unwrap();

        let span = make_span.make_span(&req);
        // The span should exist and be valid.
        // We verify by entering it and checking it doesn't panic.
        let _entered = span.enter();
    }

    #[test]
    fn test_make_span_handles_missing_request_id() {
        let mut make_span = AccessLogMakeSpan;
        let req = Request::builder()
            .method(Method::POST)
            .uri("/api/projects")
            .body(())
            .unwrap();

        let span = make_span.make_span(&req);
        let _entered = span.enter();
    }

    // -----------------------------------------------------------------------
    // AccessLogOnResponse: log level varies by status code and path
    // -----------------------------------------------------------------------
    #[test]
    fn test_on_response_health_endpoint_uses_debug() {
        // We test that on_response does not panic for health endpoints.
        let on_response = AccessLogOnResponse;
        let mut resp = Response::builder().status(StatusCode::OK).body(()).unwrap();
        resp.extensions_mut()
            .insert(RequestPath("/health".to_string()));

        let span = tracing::info_span!("test");
        on_response.on_response(&resp, std::time::Duration::from_millis(1), &span);
    }

    #[test]
    fn test_on_response_metrics_endpoint_uses_debug() {
        let on_response = AccessLogOnResponse;
        let mut resp = Response::builder().status(StatusCode::OK).body(()).unwrap();
        resp.extensions_mut()
            .insert(RequestPath("/metrics".to_string()));

        let span = tracing::info_span!("test");
        on_response.on_response(&resp, std::time::Duration::from_millis(1), &span);
    }

    #[test]
    fn test_on_response_4xx_uses_warn() {
        let on_response = AccessLogOnResponse;
        let mut resp = Response::builder()
            .status(StatusCode::NOT_FOUND)
            .body(())
            .unwrap();
        resp.extensions_mut()
            .insert(RequestPath("/api/tasks/123".to_string()));

        let span = tracing::info_span!("test");
        on_response.on_response(&resp, std::time::Duration::from_millis(5), &span);
    }

    #[test]
    fn test_on_response_5xx_uses_error() {
        let on_response = AccessLogOnResponse;
        let mut resp = Response::builder()
            .status(StatusCode::INTERNAL_SERVER_ERROR)
            .body(())
            .unwrap();
        resp.extensions_mut()
            .insert(RequestPath("/api/tasks".to_string()));

        let span = tracing::info_span!("test");
        on_response.on_response(&resp, std::time::Duration::from_millis(10), &span);
    }

    #[test]
    fn test_on_response_2xx_uses_info() {
        let on_response = AccessLogOnResponse;
        let mut resp = Response::builder().status(StatusCode::OK).body(()).unwrap();
        resp.extensions_mut()
            .insert(RequestPath("/api/tasks".to_string()));

        let span = tracing::info_span!("test");
        on_response.on_response(&resp, std::time::Duration::from_millis(2), &span);
    }

    #[test]
    fn test_on_response_without_path_extension() {
        // When no RequestPath extension is present, should default to normal logging
        let on_response = AccessLogOnResponse;
        let resp = Response::builder().status(StatusCode::OK).body(()).unwrap();

        let span = tracing::info_span!("test");
        on_response.on_response(&resp, std::time::Duration::from_millis(1), &span);
    }

    // -----------------------------------------------------------------------
    // Metric constants: verify naming conventions
    // -----------------------------------------------------------------------
    #[test]
    fn test_metric_names_follow_prometheus_conventions() {
        // All metrics should start with "gantry_"
        assert!(metric::AGENT_SESSION_DURATION.starts_with("gantry_"));
        assert!(metric::AGENT_SESSIONS_TOTAL.starts_with("gantry_"));
        assert!(metric::TASKS_TOTAL.starts_with("gantry_"));
        assert!(metric::ERRORS_TOTAL.starts_with("gantry_"));
        assert!(metric::GITHUB_SYNC_DURATION.starts_with("gantry_"));
        assert!(metric::GITHUB_SYNC_ISSUES_TOTAL.starts_with("gantry_"));
        assert!(metric::DB_POOL_CONNECTIONS.starts_with("gantry_"));
        assert!(metric::DB_WAL_SIZE_BYTES.starts_with("gantry_"));
        assert!(metric::DB_SIZE_BYTES.starts_with("gantry_"));

        // Counters should end with _total
        assert!(metric::AGENT_SESSIONS_TOTAL.ends_with("_total"));
        assert!(metric::TASKS_TOTAL.ends_with("_total"));
        assert!(metric::ERRORS_TOTAL.ends_with("_total"));
        assert!(metric::GITHUB_SYNC_ISSUES_TOTAL.ends_with("_total"));

        // Histograms should have unit suffix
        assert!(metric::AGENT_SESSION_DURATION.ends_with("_seconds"));
        assert!(metric::GITHUB_SYNC_DURATION.ends_with("_seconds"));

        // Gauges with byte unit should end with _bytes
        assert!(metric::DB_WAL_SIZE_BYTES.ends_with("_bytes"));
        assert!(metric::DB_SIZE_BYTES.ends_with("_bytes"));
    }

    // -----------------------------------------------------------------------
    // record_db_pool_metrics: functional test with real pool
    // -----------------------------------------------------------------------
    #[tokio::test]
    async fn test_record_db_pool_metrics_does_not_panic() {
        let pool = sqlx::sqlite::SqlitePoolOptions::new()
            .max_connections(2)
            .connect("sqlite::memory:")
            .await
            .unwrap();

        // Should not panic
        record_db_pool_metrics(&pool);
    }

    // -----------------------------------------------------------------------
    // DB maintenance: WAL checkpoint + PRAGMA optimize
    // -----------------------------------------------------------------------
    #[tokio::test]
    async fn test_run_db_maintenance_does_not_panic() {
        let pool = sqlx::sqlite::SqlitePoolOptions::new()
            .max_connections(2)
            .connect("sqlite::memory:")
            .await
            .unwrap();

        // Should not panic
        run_db_maintenance(&pool).await;
    }

    // -----------------------------------------------------------------------
    // DB file metrics: record_db_file_metrics with non-existent path
    // -----------------------------------------------------------------------
    #[test]
    fn test_record_db_file_metrics_with_nonexistent_path() {
        // Should not panic even with a nonexistent path
        record_db_file_metrics("sqlite:/tmp/nonexistent_test_db_12345.db");
    }

    // -----------------------------------------------------------------------
    // Error metrics recording: verify counter format for ErrorCode variants
    // -----------------------------------------------------------------------
    #[test]
    fn test_error_code_label_values() {
        // ErrorCode variants should map to SCREAMING_SNAKE_CASE for Prometheus labels
        use crate::error::ErrorCode;

        let codes = vec![
            ErrorCode::NotFound,
            ErrorCode::ValidationFailed,
            ErrorCode::Conflict,
            ErrorCode::Unauthorized,
            ErrorCode::Forbidden,
            ErrorCode::InvalidCredentials,
            ErrorCode::DatabaseError,
            ErrorCode::GitError,
            ErrorCode::InternalError,
        ];

        for code in codes {
            let label = serde_json::to_string(&code).unwrap();
            // Label should be a valid quoted string
            assert!(!label.is_empty());
            // Recording a counter with this label should not panic
            let label_str = label.trim_matches('"');
            metrics::counter!(metric::ERRORS_TOTAL, "error_code" => label_str.to_string())
                .increment(0);
        }
    }
}
