CREATE TABLE intake_session_answers (
    id TEXT PRIMARY KEY NOT NULL,
    session_id TEXT NOT NULL,
    field_code TEXT NOT NULL,
    raw_answer TEXT NOT NULL,
    candidate_value TEXT NOT NULL,
    source TEXT NOT NULL CHECK (source IN ('family_provided', 'ai_extracted')),
    status TEXT NOT NULL CHECK (status = 'draft'),
    generated_at TEXT NOT NULL,
    model TEXT,
    template_version TEXT,
    created_at TEXT NOT NULL,
    updated_at TEXT NOT NULL,
    FOREIGN KEY (session_id) REFERENCES intake_sessions(id) ON DELETE CASCADE,
    UNIQUE (session_id, field_code)
);
-- statement-break
CREATE INDEX idx_intake_session_answers_session ON intake_session_answers(session_id, created_at);
