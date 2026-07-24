CREATE TABLE user_global_capabilities (
    user_id VARCHAR(36) NOT NULL REFERENCES users(id) ON DELETE CASCADE ON UPDATE CASCADE,
    capability VARCHAR(32) NOT NULL CHECK (capability IN ('commander', 'volunteer', 'admin')),
    created_at VARCHAR(40) NOT NULL,
    PRIMARY KEY (user_id, capability)
);
-- statement-break
INSERT INTO user_global_capabilities (user_id, capability, created_at)
SELECT id, role, updated_at FROM users WHERE role IN ('commander', 'volunteer', 'admin');
-- statement-break
ALTER TABLE users ADD COLUMN account_type VARCHAR(16);
-- statement-break
UPDATE users SET account_type = CASE WHEN role = 'learner' THEN 'learner' ELSE 'member' END;
-- statement-break
ALTER TABLE users ALTER COLUMN account_type SET NOT NULL;
-- statement-break
ALTER TABLE users DROP CONSTRAINT IF EXISTS users_role_check;
-- statement-break
ALTER TABLE users DROP COLUMN role;
-- statement-break
ALTER TABLE users ADD CONSTRAINT users_account_type_check CHECK (account_type IN ('member', 'learner'));
-- statement-break
CREATE INDEX idx_users_account_type_status ON users(account_type, status);
-- statement-break
CREATE INDEX idx_user_global_capabilities_capability ON user_global_capabilities(capability, user_id);
