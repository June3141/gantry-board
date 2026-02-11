-- Prevent concurrent active sessions per task at the database level.
-- Only one session with status 'pending' or 'running' is allowed per task_id.
CREATE UNIQUE INDEX IF NOT EXISTS idx_unique_active_session_per_task
ON agent_sessions(task_id) WHERE status IN ('pending', 'running');
