ALTER TABLE intake_sessions DROP COLUMN ai_initial_reviewed_at;
-- statement-break
ALTER TABLE intake_sessions DROP COLUMN ai_initial_review_profile_json;
-- statement-break
ALTER TABLE intake_sessions DROP COLUMN ai_initial_review_json;
-- statement-break
ALTER TABLE intake_sessions DROP COLUMN ai_initial_review_status;
-- statement-break
ALTER TABLE intake_sessions DROP CONSTRAINT intake_sessions_status_check;
-- statement-break
ALTER TABLE intake_sessions ADD CONSTRAINT intake_sessions_status_check CHECK (status IN ('collecting', 'ready_for_confirmation', 'confirmed', 'closed'));
