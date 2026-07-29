ALTER TABLE users DROP CONSTRAINT IF EXISTS users_status_check;
-- statement-break
ALTER TABLE users ADD CONSTRAINT users_status_check CHECK (status IN ('active', 'disabled'));
