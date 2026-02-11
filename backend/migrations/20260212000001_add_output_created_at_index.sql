-- Index on created_at for time-based cleanup queries
CREATE INDEX IF NOT EXISTS idx_agent_session_outputs_created_at
    ON agent_session_outputs(created_at);
