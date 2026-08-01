DROP INDEX IF EXISTS idx_clue_drafts_case_review;
-- statement-break
ALTER TABLE clue_drafts DROP COLUMN candidate_json;
-- statement-break
ALTER TABLE clue_drafts DROP COLUMN review_status;
-- statement-break
ALTER TABLE clue_drafts DROP COLUMN reviewed_by_user_id;
-- statement-break
ALTER TABLE clue_drafts DROP COLUMN reviewed_at;
-- statement-break
ALTER TABLE clue_drafts DROP COLUMN review_reason;
-- statement-break
ALTER TABLE clue_drafts DROP COLUMN version;
