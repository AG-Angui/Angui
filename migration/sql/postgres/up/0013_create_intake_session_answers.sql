CREATE TABLE intake_session_answers (
    id VARCHAR(36) PRIMARY KEY,
    session_id VARCHAR(36) NOT NULL,
    field_code VARCHAR(64) NOT NULL,
    raw_answer TEXT NOT NULL,
    candidate_value TEXT NOT NULL,
    source VARCHAR(32) NOT NULL CHECK (source IN ('family_provided', 'ai_extracted')),
    status VARCHAR(32) NOT NULL CHECK (status = 'draft'),
    generated_at VARCHAR(40) NOT NULL,
    model VARCHAR(255),
    template_version VARCHAR(255),
    created_at VARCHAR(40) NOT NULL,
    updated_at VARCHAR(40) NOT NULL,
    CONSTRAINT fk_intake_session_answers_session FOREIGN KEY (session_id) REFERENCES intake_sessions(id) ON DELETE CASCADE,
    UNIQUE (session_id, field_code)
);
-- statement-break
CREATE INDEX idx_intake_session_answers_session ON intake_session_answers(session_id, created_at);
