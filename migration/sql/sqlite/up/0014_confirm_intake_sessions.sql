ALTER TABLE intake_sessions ADD COLUMN confirmed_by_user_id TEXT REFERENCES users(id) ON DELETE RESTRICT;
-- statement-break
ALTER TABLE intake_sessions ADD COLUMN confirmed_at TEXT;
