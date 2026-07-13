DROP INDEX idx_case_memberships_case_role ON case_memberships;
-- statement-break
DROP INDEX idx_case_memberships_user_id ON case_memberships;
-- statement-break
DROP TABLE IF EXISTS case_memberships;
