-- Add partial unique index on docker_previews.port to prevent race conditions
-- in port allocation. Only active previews with a reserved port are constrained,
-- matching the allocation query in allocate_port_tx().
CREATE UNIQUE INDEX IF NOT EXISTS idx_docker_previews_port_unique
ON docker_previews(port) WHERE port IS NOT NULL AND status IN ('pending', 'building', 'running');
