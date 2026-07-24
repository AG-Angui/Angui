PRAGMA foreign_keys = OFF;
-- statement-break
CREATE TABLE users_with_single_role (
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
INSERT INTO users_with_single_role (id, email, display_name, role, password_hash, status, created_at, updated_at)
SELECT u.id, u.email, u.display_name,
    CASE
        WHEN u.account_type = 'learner' THEN 'learner'
        WHEN EXISTS (SELECT 1 FROM user_global_capabilities c WHERE c.user_id = u.id AND c.capability = 'admin') THEN 'admin'
        WHEN EXISTS (SELECT 1 FROM user_global_capabilities c WHERE c.user_id = u.id AND c.capability = 'commander') THEN 'commander'
        WHEN EXISTS (SELECT 1 FROM user_global_capabilities c WHERE c.user_id = u.id AND c.capability = 'volunteer') THEN 'volunteer'
        ELSE 'family'
    END,
    u.password_hash, u.status, u.created_at, u.updated_at
FROM users u;
-- statement-break
DROP TABLE users;
-- statement-break
ALTER TABLE users_with_single_role RENAME TO users;
-- statement-break
DROP TABLE user_global_capabilities;
-- statement-break
CREATE INDEX idx_users_role_status ON users(role, status);
-- statement-break
PRAGMA foreign_keys = ON;
