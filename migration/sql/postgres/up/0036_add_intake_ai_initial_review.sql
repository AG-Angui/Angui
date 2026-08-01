ALTER TABLE intake_sessions ADD COLUMN ai_initial_review_status VARCHAR(48) NOT NULL DEFAULT 'not_started';
-- statement-break
ALTER TABLE intake_sessions ADD COLUMN ai_initial_review_json TEXT NOT NULL DEFAULT '[]';
-- statement-break
ALTER TABLE intake_sessions ADD COLUMN ai_initial_review_profile_json TEXT;
-- statement-break
ALTER TABLE intake_sessions ADD COLUMN ai_initial_reviewed_at TEXT;
-- statement-break
ALTER TABLE intake_sessions DROP CONSTRAINT intake_sessions_status_check;
-- statement-break
ALTER TABLE intake_sessions ADD CONSTRAINT intake_sessions_status_check CHECK (status IN ('collecting', 'ready_for_confirmation', 'awaiting_family_review', 'ready_for_second_confirmation', 'confirmed', 'closed'));
