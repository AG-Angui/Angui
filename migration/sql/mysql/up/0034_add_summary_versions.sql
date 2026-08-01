ALTER TABLE summary_drafts ADD COLUMN parent_draft_id VARCHAR(36) NULL, ADD COLUMN version INT NOT NULL DEFAULT 1;
-- statement-break
ALTER TABLE summary_drafts ADD CONSTRAINT chk_summary_drafts_version CHECK (version >= 1);
-- statement-break
CREATE INDEX idx_summary_drafts_case_version ON summary_drafts(case_id, version, created_at);
