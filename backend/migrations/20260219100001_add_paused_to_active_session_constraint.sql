-- Include 'paused' in the active session constraint so that paused sessions
-- also prevent a second session from being started on the same task.
DROP INDEX IF EXISTS idx_unique_active_session_per_task;

CREATE UNIQUE INDEX idx_unique_active_session_per_task
ON agent_sessions(task_id) WHERE status IN ('pending', 'running', 'paused');
