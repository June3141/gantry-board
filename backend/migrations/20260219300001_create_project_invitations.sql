CREATE TABLE IF NOT EXISTS project_invitations (
    id TEXT PRIMARY KEY NOT NULL,
    project_id TEXT NOT NULL REFERENCES projects(id) ON DELETE CASCADE,
    invited_by TEXT NOT NULL REFERENCES users(id),
    token_hash TEXT NOT NULL,
    role TEXT NOT NULL DEFAULT 'member',
    expires_at TEXT NOT NULL,
    accepted_at TEXT,
    accepted_by TEXT REFERENCES users(id),
    created_at TEXT NOT NULL DEFAULT (strftime('%Y-%m-%dT%H:%M:%fZ', 'now'))
);

CREATE INDEX IF NOT EXISTS idx_project_invitations_project_id ON project_invitations(project_id);
CREATE UNIQUE INDEX IF NOT EXISTS idx_project_invitations_token_hash ON project_invitations(token_hash);
