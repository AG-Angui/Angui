CREATE TABLE clue_drafts (
    id TEXT PRIMARY KEY NOT NULL,
    case_id TEXT NOT NULL,
    status TEXT NOT NULL CHECK (status IN ('draft', 'pending_review')),
    content TEXT NOT NULL,
    source_type TEXT NOT NULL CHECK (source_type IN ('manual_report', 'field_report')),
    raw_record_reference TEXT,
    uncertainty_notice TEXT NOT NULL,
    template_version TEXT NOT NULL,
    provider_model TEXT,
    degradation_status TEXT NOT NULL CHECK (degradation_status IN ('manual_review_required', 'rule_based_fallback')),
    created_by_user_id TEXT NOT NULL,
    created_at TEXT NOT NULL,
    updated_at TEXT NOT NULL,
    FOREIGN KEY (case_id) REFERENCES cases(id) ON DELETE CASCADE,
    FOREIGN KEY (created_by_user_id) REFERENCES users(id) ON DELETE RESTRICT
);
-- statement-break
CREATE INDEX idx_clue_drafts_case_created ON clue_drafts(case_id, created_at);
