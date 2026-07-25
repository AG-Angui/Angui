ALTER TABLE intake_sessions ADD COLUMN confirmed_by_user_id VARCHAR(36);
-- statement-break
-- Existing rows predate this nullable column; NOT VALID avoids a deployment-time table scan while enforcing all new writes.
ALTER TABLE intake_sessions ADD CONSTRAINT fk_intake_sessions_confirmer FOREIGN KEY (confirmed_by_user_id) REFERENCES users(id) ON DELETE RESTRICT NOT VALID;
-- statement-break
ALTER TABLE intake_sessions ADD COLUMN confirmed_at VARCHAR(40);
