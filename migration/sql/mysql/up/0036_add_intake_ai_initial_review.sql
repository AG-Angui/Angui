ALTER TABLE intake_sessions ADD COLUMN ai_initial_review_status VARCHAR(48) NOT NULL DEFAULT 'not_started';
-- statement-break
ALTER TABLE intake_sessions ADD COLUMN ai_initial_review_json TEXT NULL;
-- statement-break
UPDATE intake_sessions SET ai_initial_review_json = '[]' WHERE ai_initial_review_json IS NULL;
-- statement-break
ALTER TABLE intake_sessions MODIFY COLUMN ai_initial_review_json TEXT NOT NULL;
-- statement-break
ALTER TABLE intake_sessions ADD COLUMN ai_initial_review_profile_json TEXT NULL;
-- statement-break
ALTER TABLE intake_sessions ADD COLUMN ai_initial_reviewed_at TEXT NULL;
-- statement-break
ALTER TABLE intake_sessions DROP CHECK chk_intake_sessions_status;
-- statement-break
ALTER TABLE intake_sessions ADD CONSTRAINT chk_intake_sessions_status CHECK (status IN ('collecting', 'ready_for_confirmation', 'awaiting_family_review', 'ready_for_second_confirmation', 'confirmed', 'closed'));
