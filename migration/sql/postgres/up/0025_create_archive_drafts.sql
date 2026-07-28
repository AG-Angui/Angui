CREATE TABLE archive_drafts (
    id VARCHAR(36) PRIMARY KEY,
    case_id VARCHAR(36) NOT NULL,
    status VARCHAR(32) NOT NULL CHECK (status IN ('draft')),
    content TEXT NOT NULL,
    source_scope_json TEXT NOT NULL,
    deidentification_status VARCHAR(64) NOT NULL CHECK (deidentification_status IN ('manual_review_required')),
    template_version VARCHAR(128) NOT NULL,
    provider_model VARCHAR(255),
    created_by_user_id VARCHAR(36) NOT NULL,
    created_at VARCHAR(40) NOT NULL,
    updated_at VARCHAR(40) NOT NULL,
    FOREIGN KEY (case_id) REFERENCES cases(id) ON DELETE CASCADE,
    FOREIGN KEY (created_by_user_id) REFERENCES users(id) ON DELETE RESTRICT
);
-- statement-break
CREATE INDEX idx_archive_drafts_case_created ON archive_drafts(case_id, created_at);
