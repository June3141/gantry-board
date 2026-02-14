CREATE TABLE github_pull_requests (
    id TEXT PRIMARY KEY,
    github_link_id TEXT NOT NULL REFERENCES github_links(id) ON DELETE CASCADE,
    task_id TEXT NOT NULL REFERENCES tasks(id) ON DELETE CASCADE,
    pr_number INTEGER NOT NULL,
    title TEXT NOT NULL,
    url TEXT NOT NULL,
    state TEXT NOT NULL DEFAULT 'open',
    is_merged BOOLEAN NOT NULL DEFAULT 0,
    author TEXT,
    created_at TEXT NOT NULL DEFAULT (strftime('%Y-%m-%dT%H:%M:%fZ', 'now')),
    updated_at TEXT NOT NULL DEFAULT (strftime('%Y-%m-%dT%H:%M:%fZ', 'now')),
    UNIQUE(github_link_id, pr_number, task_id)
);

CREATE INDEX IF NOT EXISTS github_pull_requests_task_id_idx
    ON github_pull_requests(task_id);
