CREATE TABLE summary_drafts (
    id VARCHAR(36) PRIMARY KEY, case_id VARCHAR(36) NOT NULL, status VARCHAR(16) NOT NULL CHECK (status IN ('draft', 'pending_review', 'published', 'rejected', 'withdrawn', 'superseded')),
    content TEXT NOT NULL, source_scope_json TEXT NOT NULL, template_version VARCHAR(128) NOT NULL, provider_model VARCHAR(255), publication_eligible BOOLEAN NOT NULL, generated_by_user_id VARCHAR(36) NOT NULL,
    reviewed_by_user_id VARCHAR(36), reviewed_at VARCHAR(40), review_reason TEXT, created_at VARCHAR(40) NOT NULL, updated_at VARCHAR(40) NOT NULL,
    FOREIGN KEY (case_id) REFERENCES cases(id) ON DELETE CASCADE, FOREIGN KEY (generated_by_user_id) REFERENCES users(id) ON DELETE RESTRICT, FOREIGN KEY (reviewed_by_user_id) REFERENCES users(id) ON DELETE SET NULL
);
-- statement-break
CREATE INDEX idx_summary_drafts_case_created ON summary_drafts(case_id, created_at);
