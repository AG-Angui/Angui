PRAGMA foreign_keys = OFF;
-- statement-break
CREATE TABLE user_global_capabilities (
    user_id TEXT NOT NULL,
    capability TEXT NOT NULL CHECK (capability IN ('commander', 'volunteer', 'admin')),
    created_at TEXT NOT NULL,
    PRIMARY KEY (user_id, capability),
    FOREIGN KEY (user_id) REFERENCES users(id) ON DELETE CASCADE ON UPDATE CASCADE
);
-- statement-break
INSERT INTO user_global_capabilities (user_id, capability, created_at)
SELECT id, role, updated_at FROM users WHERE role IN ('commander', 'volunteer', 'admin');
-- statement-break
CREATE TABLE users_with_account_type (
    id TEXT PRIMARY KEY,
    email TEXT NOT NULL UNIQUE,
    display_name TEXT NOT NULL,
    account_type TEXT NOT NULL CHECK (account_type IN ('member', 'learner')),
    password_hash TEXT NOT NULL,
    status TEXT NOT NULL CHECK (status IN ('active', 'disabled')),
    created_at TEXT NOT NULL,
    updated_at TEXT NOT NULL
);
-- statement-break
INSERT INTO users_with_account_type (id, email, display_name, account_type, password_hash, status, created_at, updated_at)
SELECT id, email, display_name, CASE WHEN role = 'learner' THEN 'learner' ELSE 'member' END, password_hash, status, created_at, updated_at FROM users;
-- statement-break
DROP TABLE users;
-- statement-break
ALTER TABLE users_with_account_type RENAME TO users;
-- statement-break
CREATE INDEX idx_users_account_type_status ON users(account_type, status);
-- statement-break
CREATE INDEX idx_user_global_capabilities_capability ON user_global_capabilities(capability, user_id);
-- statement-break
PRAGMA foreign_keys = ON;
