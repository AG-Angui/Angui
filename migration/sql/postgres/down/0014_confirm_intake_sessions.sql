-- Destructive rollback: m0014 refuses this script when confirmation audit data exists.
ALTER TABLE intake_sessions DROP COLUMN confirmed_at;
-- statement-break
ALTER TABLE intake_sessions DROP COLUMN confirmed_by_user_id;
