-- Add prompt column to agent_sessions for restart support
ALTER TABLE agent_sessions ADD COLUMN prompt TEXT;
