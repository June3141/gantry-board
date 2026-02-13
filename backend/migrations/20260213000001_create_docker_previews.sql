CREATE TABLE IF NOT EXISTS docker_previews (
    id TEXT PRIMARY KEY NOT NULL,
    worktree_name TEXT NOT NULL UNIQUE,
    container_id TEXT,
    port INTEGER,
    status TEXT NOT NULL DEFAULT 'pending',
    preview_url TEXT,
    error_message TEXT,
    created_at TEXT NOT NULL DEFAULT (strftime('%Y-%m-%dT%H:%M:%fZ', 'now')),
    updated_at TEXT NOT NULL DEFAULT (strftime('%Y-%m-%dT%H:%M:%fZ', 'now'))
);

CREATE INDEX IF NOT EXISTS idx_docker_previews_worktree ON docker_previews(worktree_name);
CREATE INDEX IF NOT EXISTS idx_docker_previews_status ON docker_previews(status);
