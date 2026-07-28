CREATE TABLE archive_drafts (
    id TEXT PRIMARY KEY NOT NULL,
    case_id TEXT NOT NULL,
    status TEXT NOT NULL CHECK (status IN ('draft')),
    content TEXT NOT NULL,
    source_scope_json TEXT NOT NULL,
    deidentification_status TEXT NOT NULL CHECK (deidentification_status IN ('manual_review_required')),
    template_version TEXT NOT NULL,
    provider_model TEXT,
    created_by_user_id TEXT NOT NULL,
    created_at TEXT NOT NULL,
    updated_at TEXT NOT NULL,
    FOREIGN KEY (case_id) REFERENCES cases(id) ON DELETE CASCADE,
    FOREIGN KEY (created_by_user_id) REFERENCES users(id) ON DELETE RESTRICT
);
-- statement-break
CREATE INDEX idx_archive_drafts_case_created ON archive_drafts(case_id, created_at);
