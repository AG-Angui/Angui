CREATE TABLE archive_drafts (
    id VARCHAR(36) PRIMARY KEY,
    case_id VARCHAR(36) NOT NULL,
    status VARCHAR(32) NOT NULL,
    content TEXT NOT NULL,
    source_scope_json TEXT NOT NULL,
    deidentification_status VARCHAR(64) NOT NULL,
    template_version VARCHAR(128) NOT NULL,
    provider_model VARCHAR(255),
    created_by_user_id VARCHAR(36) NOT NULL,
    created_at VARCHAR(40) NOT NULL,
    updated_at VARCHAR(40) NOT NULL,
    CONSTRAINT chk_archive_drafts_status CHECK (status IN ('draft')),
    CONSTRAINT chk_archive_drafts_deidentification_status CHECK (deidentification_status IN ('manual_review_required')),
    CONSTRAINT fk_archive_drafts_case FOREIGN KEY (case_id) REFERENCES cases(id) ON DELETE CASCADE,
    CONSTRAINT fk_archive_drafts_creator FOREIGN KEY (created_by_user_id) REFERENCES users(id) ON DELETE RESTRICT
) ENGINE=InnoDB;
-- statement-break
CREATE INDEX idx_archive_drafts_case_created ON archive_drafts(case_id, created_at);
