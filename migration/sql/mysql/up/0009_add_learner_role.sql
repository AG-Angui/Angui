ALTER TABLE users DROP CHECK users_chk_1;
-- statement-break
ALTER TABLE users ADD CONSTRAINT users_role_check CHECK (role IN ('family', 'commander', 'volunteer', 'learner', 'admin'));
