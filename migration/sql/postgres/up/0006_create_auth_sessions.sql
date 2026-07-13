CREATE TABLE auth_sessions (
    id VARCHAR(36) PRIMARY KEY,
    user_id VARCHAR(36) NOT NULL,
    token_hash CHAR(64) NOT NULL UNIQUE,
    expires_at VARCHAR(40) NOT NULL,
    revoked_at VARCHAR(40),
    created_at VARCHAR(40) NOT NULL,
    last_used_at VARCHAR(40) NOT NULL,
    CONSTRAINT fk_auth_sessions_user FOREIGN KEY (user_id) REFERENCES users(id) ON DELETE CASCADE
);
-- statement-break
CREATE INDEX idx_auth_sessions_user_id ON auth_sessions(user_id);
-- statement-break
CREATE INDEX idx_auth_sessions_expires_at ON auth_sessions(expires_at);
