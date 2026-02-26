-- Add per-project repository path for worktree management.
-- When NULL, the global config.repository_path is used as fallback.
ALTER TABLE projects ADD COLUMN repository_path TEXT;
