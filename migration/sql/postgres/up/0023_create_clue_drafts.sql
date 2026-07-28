CREATE TABLE clue_drafts (
    id VARCHAR(36) PRIMARY KEY,
    case_id VARCHAR(36) NOT NULL REFERENCES cases(id) ON DELETE CASCADE,
    status VARCHAR(32) NOT NULL CHECK (status IN ('draft', 'pending_review')),
    content TEXT NOT NULL,
    source_type VARCHAR(32) NOT NULL CHECK (source_type IN ('manual_report', 'field_report')),
    raw_record_reference TEXT,
    uncertainty_notice TEXT NOT NULL,
    template_version VARCHAR(128) NOT NULL,
    provider_model VARCHAR(255),
    degradation_status VARCHAR(64) NOT NULL CHECK (degradation_status IN ('manual_review_required', 'rule_based_fallback')),
    created_by_user_id VARCHAR(36) NOT NULL REFERENCES users(id) ON DELETE RESTRICT,
    created_at VARCHAR(40) NOT NULL,
    updated_at VARCHAR(40) NOT NULL
);
-- statement-break
CREATE INDEX idx_clue_drafts_case_created ON clue_drafts(case_id, created_at);
