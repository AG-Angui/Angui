DROP INDEX idx_summary_drafts_one_published_per_case ON summary_drafts;
-- statement-break
ALTER TABLE summary_drafts DROP COLUMN published_case_id;
