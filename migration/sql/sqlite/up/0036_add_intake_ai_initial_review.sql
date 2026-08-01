ALTER TABLE intake_sessions ADD COLUMN ai_initial_review_status TEXT NOT NULL DEFAULT 'not_started';
-- statement-break
ALTER TABLE intake_sessions ADD COLUMN ai_initial_review_json TEXT NOT NULL DEFAULT '[]';
-- statement-break
ALTER TABLE intake_sessions ADD COLUMN ai_initial_review_profile_json TEXT;
-- statement-break
ALTER TABLE intake_sessions ADD COLUMN ai_initial_reviewed_at TEXT;
-- statement-break
PRAGMA foreign_keys = OFF;
-- statement-break
CREATE TABLE intake_sessions_replacement (
    id TEXT PRIMARY KEY NOT NULL,
    created_by_user_id TEXT NOT NULL,
    case_id TEXT UNIQUE,
    question_set_version INTEGER NOT NULL,
    status TEXT NOT NULL CHECK (status IN ('collecting', 'ready_for_confirmation', 'awaiting_family_review', 'ready_for_second_confirmation', 'confirmed', 'closed')),
    answers_json TEXT NOT NULL,
    assessment_json TEXT NOT NULL DEFAULT '[]',
    structured_answers_json TEXT NOT NULL DEFAULT '{}',
    ai_initial_review_status TEXT NOT NULL DEFAULT 'not_started',
    ai_initial_review_json TEXT NOT NULL DEFAULT '[]',
    ai_initial_review_profile_json TEXT,
    ai_initial_reviewed_at TEXT,
    confirmed_by_user_id TEXT,
    confirmed_at TEXT,
    created_at TEXT NOT NULL,
    updated_at TEXT NOT NULL,
    FOREIGN KEY (created_by_user_id) REFERENCES users(id) ON DELETE RESTRICT,
    FOREIGN KEY (case_id) REFERENCES cases(id) ON DELETE SET NULL,
    FOREIGN KEY (confirmed_by_user_id) REFERENCES users(id) ON DELETE RESTRICT
);
-- statement-break
INSERT INTO intake_sessions_replacement (id, created_by_user_id, case_id, question_set_version, status, answers_json, assessment_json, structured_answers_json, ai_initial_review_status, ai_initial_review_json, ai_initial_review_profile_json, ai_initial_reviewed_at, confirmed_by_user_id, confirmed_at, created_at, updated_at)
SELECT id, created_by_user_id, case_id, question_set_version, status, answers_json, assessment_json, structured_answers_json, ai_initial_review_status, ai_initial_review_json, ai_initial_review_profile_json, ai_initial_reviewed_at, confirmed_by_user_id, confirmed_at, created_at, updated_at
FROM intake_sessions;
-- statement-break
DROP TABLE intake_sessions;
-- statement-break
ALTER TABLE intake_sessions_replacement RENAME TO intake_sessions;
-- statement-break
CREATE INDEX idx_intake_sessions_creator ON intake_sessions(created_by_user_id, created_at);
-- statement-break
PRAGMA foreign_keys = ON;
