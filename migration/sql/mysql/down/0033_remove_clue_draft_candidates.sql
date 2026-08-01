DROP INDEX idx_clue_drafts_case_review ON clue_drafts;
-- statement-break
ALTER TABLE clue_drafts DROP CHECK chk_clue_drafts_review_status, DROP CHECK chk_clue_drafts_version, DROP COLUMN candidate_json, DROP COLUMN review_status, DROP COLUMN reviewed_by_user_id, DROP COLUMN reviewed_at, DROP COLUMN review_reason, DROP COLUMN version;
