ALTER TABLE users DROP CHECK users_chk_2;
-- statement-break
ALTER TABLE users ADD CONSTRAINT users_status_check CHECK (status IN ('active', 'disabled', 'locked'));
