-- Add admin flag to users table.
-- The first registered user is retroactively promoted to admin.
ALTER TABLE users ADD COLUMN is_admin BOOLEAN NOT NULL DEFAULT FALSE;

-- Promote the earliest-registered user to admin
UPDATE users
SET is_admin = TRUE
WHERE id = (SELECT id FROM users ORDER BY created_at ASC LIMIT 1);
