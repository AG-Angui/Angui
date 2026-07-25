ALTER TABLE intake_sessions ADD COLUMN assessment_json TEXT NOT NULL DEFAULT ('[]');
-- statement-break
ALTER TABLE intake_sessions ADD COLUMN structured_answers_json TEXT NOT NULL DEFAULT ('{}');
-- statement-break
CREATE TABLE intake_answer_revisions (
    id VARCHAR(36) PRIMARY KEY, session_id VARCHAR(36) NOT NULL, field_code VARCHAR(64) NOT NULL, raw_answer TEXT NOT NULL,
    structured_json TEXT, revision_kind VARCHAR(16) NOT NULL, created_by_user_id VARCHAR(36) NOT NULL, created_at VARCHAR(40) NOT NULL,
    CONSTRAINT chk_intake_answer_revisions_kind CHECK (revision_kind IN ('submitted', 'corrected')),
    FOREIGN KEY (session_id) REFERENCES intake_sessions(id) ON DELETE CASCADE,
    FOREIGN KEY (created_by_user_id) REFERENCES users(id) ON DELETE RESTRICT
) ENGINE=InnoDB;
-- statement-break
CREATE INDEX idx_intake_answer_revisions_session ON intake_answer_revisions(session_id, created_at);
