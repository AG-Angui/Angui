ALTER TABLE clue_drafts ADD COLUMN candidate_json TEXT NOT NULL, ADD COLUMN review_status VARCHAR(32) NOT NULL DEFAULT 'pending_review', ADD COLUMN reviewed_by_user_id VARCHAR(36) NULL, ADD COLUMN reviewed_at VARCHAR(40) NULL, ADD COLUMN review_reason TEXT NULL, ADD COLUMN version INT NOT NULL DEFAULT 1, ADD CONSTRAINT chk_clue_drafts_review_status CHECK (review_status IN ('pending_review', 'accepted', 'rejected')), ADD CONSTRAINT chk_clue_drafts_version CHECK (version >= 1);
-- statement-break
UPDATE clue_drafts SET candidate_json = '{}' WHERE candidate_json = '';
-- statement-break
CREATE INDEX idx_clue_drafts_case_review ON clue_drafts(case_id, review_status, updated_at);
