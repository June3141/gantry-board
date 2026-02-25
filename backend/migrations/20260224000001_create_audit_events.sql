-- Audit events table for tracking security-relevant actions
CREATE TABLE IF NOT EXISTS audit_events (
    id TEXT PRIMARY KEY NOT NULL,
    event_type TEXT NOT NULL,          -- e.g. "auth.register", "auth.login", "task.create"
    actor_id TEXT,                      -- user ID (NULL for system events)
    target_type TEXT,                   -- e.g. "user", "task", "project", "session"
    target_id TEXT,                     -- ID of the target resource
    metadata TEXT,                      -- JSON blob for additional context
    ip_address TEXT,                    -- client IP address
    created_at TEXT NOT NULL DEFAULT (strftime('%Y-%m-%dT%H:%M:%fZ', 'now'))
);

-- Index for listing by time (DESC for most-recent-first)
CREATE INDEX IF NOT EXISTS idx_audit_events_created_at ON audit_events(created_at DESC);

-- Index for filtering by event_type
CREATE INDEX IF NOT EXISTS idx_audit_events_event_type ON audit_events(event_type);

-- Index for filtering by actor
CREATE INDEX IF NOT EXISTS idx_audit_events_actor_id ON audit_events(actor_id);
