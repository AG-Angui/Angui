PRAGMA foreign_keys = OFF;
-- statement-break
CREATE TABLE intake_sessions_replacement (
    id TEXT PRIMARY KEY NOT NULL,
    created_by_user_id TEXT NOT NULL,
    case_id TEXT UNIQUE,
    question_set_version INTEGER NOT NULL,
    status TEXT NOT NULL CHECK (status IN ('collecting', 'ready_for_confirmation', 'confirmed', 'closed')),
    answers_json TEXT NOT NULL,
    assessment_json TEXT NOT NULL DEFAULT '[]',
    structured_answers_json TEXT NOT NULL DEFAULT '{}',
    confirmed_by_user_id TEXT,
    confirmed_at TEXT,
    created_at TEXT NOT NULL,
    updated_at TEXT NOT NULL,
    FOREIGN KEY (created_by_user_id) REFERENCES users(id) ON DELETE RESTRICT,
    FOREIGN KEY (case_id) REFERENCES cases(id) ON DELETE SET NULL,
    FOREIGN KEY (confirmed_by_user_id) REFERENCES users(id) ON DELETE RESTRICT
);
-- statement-break
INSERT INTO intake_sessions_replacement (id, created_by_user_id, case_id, question_set_version, status, answers_json, assessment_json, structured_answers_json, confirmed_by_user_id, confirmed_at, created_at, updated_at)
SELECT id, created_by_user_id, case_id, question_set_version,
       CASE WHEN status IN ('awaiting_family_review', 'ready_for_second_confirmation') THEN 'ready_for_confirmation' ELSE status END,
       answers_json, assessment_json, structured_answers_json, confirmed_by_user_id, confirmed_at, created_at, updated_at
FROM intake_sessions;
-- statement-break
DROP TABLE intake_sessions;
-- statement-break
ALTER TABLE intake_sessions_replacement RENAME TO intake_sessions;
-- statement-break
PRAGMA foreign_keys = ON;
