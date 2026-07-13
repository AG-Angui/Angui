CREATE TABLE auth_sessions (
    id TEXT PRIMARY KEY,
    user_id TEXT NOT NULL,
    token_hash TEXT NOT NULL UNIQUE,
    expires_at TEXT NOT NULL,
    revoked_at TEXT,
    created_at TEXT NOT NULL,
    last_used_at TEXT NOT NULL,
    FOREIGN KEY (user_id) REFERENCES users(id) ON DELETE CASCADE
);
-- statement-break
CREATE INDEX idx_auth_sessions_user_id ON auth_sessions(user_id);
-- statement-break
CREATE INDEX idx_auth_sessions_expires_at ON auth_sessions(expires_at);
