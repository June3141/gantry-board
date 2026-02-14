-- GitHub integration tables

CREATE TABLE IF NOT EXISTS github_links (
    id TEXT PRIMARY KEY NOT NULL,
    project_id TEXT NOT NULL UNIQUE REFERENCES projects(id) ON DELETE CASCADE,
    repo_owner TEXT NOT NULL,
    repo_name TEXT NOT NULL,
    sync_enabled BOOLEAN NOT NULL DEFAULT 1,
    last_synced_at TEXT,
    created_at TEXT NOT NULL DEFAULT (strftime('%Y-%m-%dT%H:%M:%fZ', 'now')),
    updated_at TEXT NOT NULL DEFAULT (strftime('%Y-%m-%dT%H:%M:%fZ', 'now'))
);

CREATE INDEX IF NOT EXISTS idx_github_links_project_id ON github_links(project_id);

CREATE TABLE IF NOT EXISTS github_issue_mappings (
    id TEXT PRIMARY KEY NOT NULL,
    task_id TEXT NOT NULL UNIQUE REFERENCES tasks(id) ON DELETE CASCADE,
    github_link_id TEXT NOT NULL REFERENCES github_links(id) ON DELETE CASCADE,
    github_issue_number INTEGER NOT NULL,
    github_issue_id INTEGER,
    last_local_update TEXT,
    last_remote_update TEXT,
    created_at TEXT NOT NULL DEFAULT (strftime('%Y-%m-%dT%H:%M:%fZ', 'now')),
    UNIQUE(github_link_id, github_issue_number)
);

CREATE INDEX IF NOT EXISTS idx_github_issue_mappings_task_id ON github_issue_mappings(task_id);
CREATE INDEX IF NOT EXISTS idx_github_issue_mappings_link_id ON github_issue_mappings(github_link_id);
