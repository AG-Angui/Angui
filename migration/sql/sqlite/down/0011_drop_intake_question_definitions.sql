DROP TABLE IF EXISTS intake_question_definitions;
-- statement-break
PRAGMA foreign_keys = OFF;
-- statement-break
CREATE TABLE intake_sessions_without_question_set_version (
    id TEXT PRIMARY KEY NOT NULL,
    created_by_user_id TEXT NOT NULL,
    case_id TEXT,
    status TEXT NOT NULL CHECK (status IN ('collecting', 'ready_for_confirmation', 'confirmed', 'closed')),
    answers_json TEXT NOT NULL,
    created_at TEXT NOT NULL,
    updated_at TEXT NOT NULL,
    FOREIGN KEY (created_by_user_id) REFERENCES users(id) ON DELETE RESTRICT,
    FOREIGN KEY (case_id) REFERENCES cases(id) ON DELETE SET NULL,
    UNIQUE (case_id)
);
-- statement-break
INSERT INTO intake_sessions_without_question_set_version (id, created_by_user_id, case_id, status, answers_json, created_at, updated_at)
SELECT id, created_by_user_id, case_id, status, answers_json, created_at, updated_at FROM intake_sessions;
-- statement-break
DROP TABLE intake_sessions;
-- statement-break
ALTER TABLE intake_sessions_without_question_set_version RENAME TO intake_sessions;
-- statement-break
CREATE INDEX idx_intake_sessions_creator ON intake_sessions(created_by_user_id, created_at);
-- statement-break
PRAGMA foreign_keys = ON;
