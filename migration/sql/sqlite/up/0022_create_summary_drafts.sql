CREATE TABLE summary_drafts (
    id TEXT PRIMARY KEY NOT NULL,
    case_id TEXT NOT NULL,
    status TEXT NOT NULL CHECK (status IN ('draft', 'pending_review', 'published', 'rejected', 'withdrawn', 'superseded')),
    content TEXT NOT NULL,
    source_scope_json TEXT NOT NULL,
    template_version TEXT NOT NULL,
    provider_model TEXT,
    publication_eligible INTEGER NOT NULL CHECK (publication_eligible IN (0, 1)),
    generated_by_user_id TEXT NOT NULL,
    reviewed_by_user_id TEXT,
    reviewed_at TEXT,
    review_reason TEXT,
    created_at TEXT NOT NULL,
    updated_at TEXT NOT NULL,
    FOREIGN KEY (case_id) REFERENCES cases(id) ON DELETE CASCADE,
    FOREIGN KEY (generated_by_user_id) REFERENCES users(id) ON DELETE RESTRICT,
    FOREIGN KEY (reviewed_by_user_id) REFERENCES users(id) ON DELETE SET NULL
);
-- statement-break
CREATE INDEX idx_summary_drafts_case_created ON summary_drafts(case_id, created_at);
