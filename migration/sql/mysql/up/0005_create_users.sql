CREATE TABLE users (
    id VARCHAR(36) PRIMARY KEY,
    email VARCHAR(320) NOT NULL UNIQUE,
    display_name VARCHAR(120) NOT NULL,
    role VARCHAR(32) NOT NULL CHECK (role IN ('family', 'commander', 'volunteer', 'admin')),
    password_hash TEXT NOT NULL,
    status VARCHAR(32) NOT NULL CHECK (status IN ('active', 'disabled')),
    created_at VARCHAR(40) NOT NULL,
    updated_at VARCHAR(40) NOT NULL
) ENGINE=InnoDB;
-- statement-break
CREATE INDEX idx_users_role_status ON users(role, status);
