PRAGMA foreign_keys = OFF;
-- statement-break
CREATE TABLE users_with_learner_role (
    id TEXT PRIMARY KEY,
    email TEXT NOT NULL UNIQUE,
    display_name TEXT NOT NULL,
    role TEXT NOT NULL CHECK (role IN ('family', 'commander', 'volunteer', 'learner', 'admin')),
    password_hash TEXT NOT NULL,
    status TEXT NOT NULL CHECK (status IN ('active', 'disabled')),
    created_at TEXT NOT NULL,
    updated_at TEXT NOT NULL
);
-- statement-break
INSERT INTO users_with_learner_role (id, email, display_name, role, password_hash, status, created_at, updated_at)
SELECT id, email, display_name, role, password_hash, status, created_at, updated_at FROM users;
-- statement-break
DROP TABLE users;
-- statement-break
ALTER TABLE users_with_learner_role RENAME TO users;
-- statement-break
CREATE INDEX idx_users_role_status ON users(role, status);
-- statement-break
PRAGMA foreign_keys = ON;
