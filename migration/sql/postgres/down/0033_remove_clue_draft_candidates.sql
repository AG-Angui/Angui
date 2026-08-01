DROP INDEX IF EXISTS idx_clue_drafts_case_review;
-- statement-break
ALTER TABLE clue_drafts DROP CONSTRAINT IF EXISTS clue_drafts_review_status_check, DROP CONSTRAINT IF EXISTS clue_drafts_version_check, DROP COLUMN candidate_json, DROP COLUMN review_status, DROP COLUMN reviewed_by_user_id, DROP COLUMN reviewed_at, DROP COLUMN review_reason, DROP COLUMN version;
