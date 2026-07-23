UPDATE users SET role = 'volunteer', status = 'disabled' WHERE role = 'learner';
-- statement-break
ALTER TABLE users DROP CONSTRAINT IF EXISTS users_role_check;
-- statement-break
ALTER TABLE users ADD CONSTRAINT users_role_check CHECK (role IN ('family', 'commander', 'volunteer', 'admin'));
