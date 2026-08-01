DROP INDEX idx_summary_drafts_case_version ON summary_drafts;
-- statement-break
ALTER TABLE summary_drafts DROP CHECK chk_summary_drafts_version;
-- statement-break
ALTER TABLE summary_drafts DROP COLUMN parent_draft_id, DROP COLUMN version;
