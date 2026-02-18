-- Add partial unique index on docker_previews.port to prevent race conditions
-- in port allocation. Only active previews (non-NULL port) are constrained.
CREATE UNIQUE INDEX IF NOT EXISTS idx_docker_previews_port_unique
ON docker_previews(port) WHERE port IS NOT NULL;
