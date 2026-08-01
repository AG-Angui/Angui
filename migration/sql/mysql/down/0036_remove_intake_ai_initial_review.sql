ALTER TABLE intake_sessions DROP COLUMN ai_initial_reviewed_at;
-- statement-break
ALTER TABLE intake_sessions DROP COLUMN ai_initial_review_profile_json;
-- statement-break
ALTER TABLE intake_sessions DROP COLUMN ai_initial_review_json;
-- statement-break
ALTER TABLE intake_sessions DROP COLUMN ai_initial_review_status;
-- statement-break
ALTER TABLE intake_sessions DROP CHECK chk_intake_sessions_status;
-- statement-break
ALTER TABLE intake_sessions ADD CONSTRAINT chk_intake_sessions_status CHECK (status IN ('collecting', 'ready_for_confirmation', 'confirmed', 'closed'));
