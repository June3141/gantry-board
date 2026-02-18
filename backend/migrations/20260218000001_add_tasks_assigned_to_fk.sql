-- Add foreign key constraint on tasks.assigned_to -> users.id
-- SQLite does not support ALTER TABLE ADD FOREIGN KEY, so we recreate the table.

PRAGMA foreign_keys = OFF;

CREATE TABLE tasks_new (
    id TEXT PRIMARY KEY NOT NULL,
    project_id TEXT NOT NULL REFERENCES projects(id) ON DELETE CASCADE,
    title TEXT NOT NULL,
    description TEXT,
    status TEXT NOT NULL DEFAULT 'backlog',
    priority TEXT NOT NULL DEFAULT 'medium',
    parent_id TEXT REFERENCES tasks_new(id) ON DELETE SET NULL,
    assigned_to TEXT REFERENCES users(id) ON DELETE SET NULL,
    position INTEGER NOT NULL DEFAULT 0,
    created_at TEXT NOT NULL DEFAULT (strftime('%Y-%m-%dT%H:%M:%fZ', 'now')),
    updated_at TEXT NOT NULL DEFAULT (strftime('%Y-%m-%dT%H:%M:%fZ', 'now'))
);

INSERT INTO tasks_new SELECT * FROM tasks;

DROP TABLE tasks;

ALTER TABLE tasks_new RENAME TO tasks;

-- Recreate indexes that existed on the original table
CREATE INDEX IF NOT EXISTS idx_tasks_project_id ON tasks(project_id);
CREATE INDEX IF NOT EXISTS idx_tasks_status ON tasks(status);
CREATE INDEX IF NOT EXISTS idx_tasks_parent_id ON tasks(parent_id);
CREATE INDEX IF NOT EXISTS idx_tasks_assigned_to ON tasks(assigned_to);

-- Recreate composite indexes from migration 20260216000001
CREATE INDEX IF NOT EXISTS idx_tasks_project_position ON tasks(project_id, position, created_at);
CREATE INDEX IF NOT EXISTS idx_tasks_project_status ON tasks(project_id, status);
CREATE INDEX IF NOT EXISTS idx_tasks_project_assigned ON tasks(project_id, assigned_to);

PRAGMA foreign_keys = ON;
