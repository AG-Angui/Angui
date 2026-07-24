ALTER TABLE users DROP CHECK users_account_type_check;
-- statement-break
ALTER TABLE users ADD COLUMN role VARCHAR(32);
-- statement-break
UPDATE users u SET role = CASE
    WHEN u.account_type = 'learner' THEN 'learner'
    WHEN EXISTS (SELECT 1 FROM user_global_capabilities c WHERE c.user_id = u.id AND c.capability = 'admin') THEN 'admin'
    WHEN EXISTS (SELECT 1 FROM user_global_capabilities c WHERE c.user_id = u.id AND c.capability = 'commander') THEN 'commander'
    WHEN EXISTS (SELECT 1 FROM user_global_capabilities c WHERE c.user_id = u.id AND c.capability = 'volunteer') THEN 'volunteer'
    ELSE 'family'
END;
-- statement-break
ALTER TABLE users MODIFY COLUMN role VARCHAR(32) NOT NULL;
-- statement-break
ALTER TABLE users DROP COLUMN account_type;
-- statement-break
DROP TABLE user_global_capabilities;
-- statement-break
ALTER TABLE users ADD CONSTRAINT users_role_check CHECK (role IN ('family', 'commander', 'volunteer', 'learner', 'admin'));
-- statement-break
CREATE INDEX idx_users_role_status ON users(role, status);
