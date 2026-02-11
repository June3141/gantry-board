CREATE TABLE IF NOT EXISTS agent_session_outputs (
    id INTEGER PRIMARY KEY AUTOINCREMENT,
    session_id TEXT NOT NULL REFERENCES agent_sessions(id) ON DELETE CASCADE,
    sequence INTEGER NOT NULL,
    content TEXT NOT NULL,
    created_at TEXT NOT NULL DEFAULT (strftime('%Y-%m-%dT%H:%M:%fZ', 'now'))
);

CREATE INDEX IF NOT EXISTS idx_agent_session_outputs_session_id ON agent_session_outputs(session_id);
CREATE UNIQUE INDEX IF NOT EXISTS idx_agent_session_outputs_session_sequence ON agent_session_outputs(session_id, sequence);
