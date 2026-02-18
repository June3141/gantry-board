-- Composite index for tasks filtered by project_id with position/created_at ordering
CREATE INDEX IF NOT EXISTS idx_tasks_project_position ON tasks(project_id, position, created_at);

-- Composite index for agent_sessions filtered by task_id with created_at ordering
CREATE INDEX IF NOT EXISTS idx_agent_sessions_task_created ON agent_sessions(task_id, created_at DESC);

-- Composite index for agent_session_outputs filtered by session_id with created_at ordering
CREATE INDEX IF NOT EXISTS idx_agent_session_outputs_session_created ON agent_session_outputs(session_id, created_at ASC);

-- Composite index for docker_previews port allocation query
CREATE INDEX IF NOT EXISTS idx_docker_previews_status_port ON docker_previews(status, port);

-- Composite index for project_members filtered by project_id
CREATE INDEX IF NOT EXISTS idx_project_members_project_user ON project_members(project_id, user_id);

-- Composite index for task_comments filtered by task_id with created_at ordering
CREATE INDEX IF NOT EXISTS idx_task_comments_task_created ON task_comments(task_id, created_at ASC);

-- Composite index for github_issue_mappings bidirectional lookups
CREATE INDEX IF NOT EXISTS idx_github_issue_mappings_link_number ON github_issue_mappings(github_link_id, github_issue_number);
