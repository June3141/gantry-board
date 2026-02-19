-- Add worktree_name column to agent_sessions so we can track which worktree
-- belongs to each session and clean it up when a PR is merged.
ALTER TABLE agent_sessions ADD COLUMN worktree_name TEXT;
