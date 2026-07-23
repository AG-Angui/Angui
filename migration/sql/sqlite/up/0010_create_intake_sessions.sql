CREATE TABLE intake_sessions (
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
CREATE INDEX idx_intake_sessions_creator ON intake_sessions(created_by_user_id, created_at);
