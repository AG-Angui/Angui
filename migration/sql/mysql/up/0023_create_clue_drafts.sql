CREATE TABLE clue_drafts (
    id VARCHAR(36) NOT NULL,
    case_id VARCHAR(36) NOT NULL,
    status VARCHAR(32) NOT NULL,
    content TEXT NOT NULL,
    source_type VARCHAR(32) NOT NULL,
    raw_record_reference TEXT NULL,
    uncertainty_notice TEXT NOT NULL,
    template_version VARCHAR(128) NOT NULL,
    provider_model VARCHAR(255) NULL,
    degradation_status VARCHAR(64) NOT NULL,
    created_by_user_id VARCHAR(36) NOT NULL,
    created_at VARCHAR(40) NOT NULL,
    updated_at VARCHAR(40) NOT NULL,
    PRIMARY KEY (id),
    CONSTRAINT chk_clue_drafts_status CHECK (status IN ('draft', 'pending_review')),
    CONSTRAINT chk_clue_drafts_source_type CHECK (source_type IN ('manual_report', 'field_report')),
    CONSTRAINT chk_clue_drafts_degradation CHECK (degradation_status IN ('manual_review_required', 'rule_based_fallback')),
    CONSTRAINT fk_clue_drafts_case FOREIGN KEY (case_id) REFERENCES cases(id) ON DELETE CASCADE,
    CONSTRAINT fk_clue_drafts_creator FOREIGN KEY (created_by_user_id) REFERENCES users(id) ON DELETE RESTRICT
) ENGINE=InnoDB;
-- statement-break
CREATE INDEX idx_clue_drafts_case_created ON clue_drafts(case_id, created_at);
