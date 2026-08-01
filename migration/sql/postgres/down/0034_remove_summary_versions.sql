DROP INDEX IF EXISTS idx_summary_drafts_case_version;
-- statement-break
ALTER TABLE summary_drafts DROP CONSTRAINT IF EXISTS summary_drafts_version_check;
-- statement-break
ALTER TABLE summary_drafts DROP COLUMN parent_draft_id, DROP COLUMN version;
