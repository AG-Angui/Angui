DROP INDEX idx_auth_sessions_expires_at ON auth_sessions;
-- statement-break
DROP INDEX idx_auth_sessions_user_id ON auth_sessions;
-- statement-break
DROP TABLE IF EXISTS auth_sessions;
