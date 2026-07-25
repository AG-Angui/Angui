DROP TABLE IF EXISTS intake_answer_revisions;
-- statement-break
PRAGMA foreign_keys = OFF;
-- statement-break
CREATE TABLE intake_sessions_before_assessment (
    id TEXT PRIMARY KEY NOT NULL, created_by_user_id TEXT NOT NULL, case_id TEXT, question_set_version INTEGER NOT NULL,
    status TEXT NOT NULL CHECK (status IN ('collecting', 'ready_for_confirmation', 'confirmed', 'closed')), answers_json TEXT NOT NULL,
    confirmed_by_user_id TEXT REFERENCES users(id) ON DELETE RESTRICT, confirmed_at TEXT, created_at TEXT NOT NULL, updated_at TEXT NOT NULL,
    FOREIGN KEY (created_by_user_id) REFERENCES users(id) ON DELETE RESTRICT, FOREIGN KEY (case_id) REFERENCES cases(id) ON DELETE SET NULL, UNIQUE (case_id)
);
-- statement-break
INSERT INTO intake_sessions_before_assessment (id, created_by_user_id, case_id, question_set_version, status, answers_json, confirmed_by_user_id, confirmed_at, created_at, updated_at) SELECT id, created_by_user_id, case_id, question_set_version, status, answers_json, confirmed_by_user_id, confirmed_at, created_at, updated_at FROM intake_sessions;
-- statement-break
DROP TABLE intake_sessions;
-- statement-break
ALTER TABLE intake_sessions_before_assessment RENAME TO intake_sessions;
-- statement-break
CREATE INDEX idx_intake_sessions_creator ON intake_sessions(created_by_user_id, created_at);
-- statement-break
PRAGMA foreign_keys = ON;
