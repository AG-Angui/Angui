CREATE TABLE intake_sessions (
    id VARCHAR(36) PRIMARY KEY,
    created_by_user_id VARCHAR(36) NOT NULL,
    case_id VARCHAR(36) UNIQUE,
    status VARCHAR(32) NOT NULL CHECK (status IN ('collecting', 'ready_for_confirmation', 'confirmed', 'closed')),
    answers_json TEXT NOT NULL,
    created_at VARCHAR(40) NOT NULL,
    updated_at VARCHAR(40) NOT NULL,
    CONSTRAINT fk_intake_sessions_creator FOREIGN KEY (created_by_user_id) REFERENCES users(id) ON DELETE RESTRICT,
    CONSTRAINT fk_intake_sessions_case FOREIGN KEY (case_id) REFERENCES cases(id) ON DELETE SET NULL
);
-- statement-break
CREATE INDEX idx_intake_sessions_creator ON intake_sessions(created_by_user_id, created_at);
