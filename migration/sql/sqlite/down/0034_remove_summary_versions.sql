DROP INDEX IF EXISTS idx_summary_drafts_case_version;
-- statement-break
ALTER TABLE summary_drafts DROP COLUMN parent_draft_id;
-- statement-break
ALTER TABLE summary_drafts DROP COLUMN version;
