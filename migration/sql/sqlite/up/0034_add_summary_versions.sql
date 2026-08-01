ALTER TABLE summary_drafts ADD COLUMN parent_draft_id TEXT;
-- statement-break
ALTER TABLE summary_drafts ADD COLUMN version INTEGER NOT NULL DEFAULT 1 CHECK (version >= 1);
-- statement-break
CREATE INDEX idx_summary_drafts_case_version ON summary_drafts(case_id, version, created_at);
