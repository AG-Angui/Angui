PRAGMA foreign_keys = OFF;
-- statement-break
CREATE TABLE users_with_locked_status (
    id TEXT PRIMARY KEY,
    email TEXT NOT NULL UNIQUE,
    display_name TEXT NOT NULL,
    account_type TEXT NOT NULL CHECK (account_type IN ('member', 'learner')),
    password_hash TEXT NOT NULL,
    status TEXT NOT NULL CHECK (status IN ('active', 'disabled', 'locked')),
    created_at TEXT NOT NULL,
    updated_at TEXT NOT NULL
);
-- statement-break
INSERT INTO users_with_locked_status (id, email, display_name, account_type, password_hash, status, created_at, updated_at)
SELECT id, email, display_name, account_type, password_hash, status, created_at, updated_at FROM users;
-- statement-break
DROP TABLE users;
-- statement-break
ALTER TABLE users_with_locked_status RENAME TO users;
-- statement-break
CREATE INDEX idx_users_account_type_status ON users(account_type, status);
-- statement-break
PRAGMA foreign_keys = ON;
