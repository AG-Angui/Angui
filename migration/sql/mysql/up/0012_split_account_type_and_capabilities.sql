CREATE TABLE user_global_capabilities (
    user_id VARCHAR(36) NOT NULL,
    capability VARCHAR(32) NOT NULL,
    created_at VARCHAR(40) NOT NULL,
    PRIMARY KEY (user_id, capability),
    CONSTRAINT fk_user_global_capabilities_user FOREIGN KEY (user_id) REFERENCES users(id) ON DELETE CASCADE ON UPDATE CASCADE,
    CONSTRAINT chk_user_global_capabilities_capability CHECK (capability IN ('commander', 'volunteer', 'admin'))
) ENGINE=InnoDB;
-- statement-break
INSERT INTO user_global_capabilities (user_id, capability, created_at)
SELECT id, role, updated_at FROM users WHERE role IN ('commander', 'volunteer', 'admin');
-- statement-break
ALTER TABLE users DROP CHECK users_role_check;
-- statement-break
UPDATE users SET role = CASE WHEN role = 'learner' THEN 'learner' ELSE 'member' END;
-- statement-break
ALTER TABLE users CHANGE COLUMN role account_type VARCHAR(16) NOT NULL;
-- statement-break
ALTER TABLE users ADD CONSTRAINT users_account_type_check CHECK (account_type IN ('member', 'learner'));
-- statement-break
CREATE INDEX idx_users_account_type_status ON users(account_type, status);
-- statement-break
CREATE INDEX idx_user_global_capabilities_capability ON user_global_capabilities(capability, user_id);
