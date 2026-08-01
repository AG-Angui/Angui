ALTER TABLE clue_drafts ADD COLUMN candidate_json TEXT NOT NULL DEFAULT '{}', ADD COLUMN review_status VARCHAR(32) NOT NULL DEFAULT 'pending_review', ADD COLUMN reviewed_by_user_id VARCHAR(36), ADD COLUMN reviewed_at VARCHAR(40), ADD COLUMN review_reason TEXT, ADD COLUMN version INTEGER NOT NULL DEFAULT 1;
-- statement-break
ALTER TABLE clue_drafts ADD CONSTRAINT clue_drafts_review_status_check CHECK (review_status IN ('pending_review', 'accepted', 'rejected')), ADD CONSTRAINT clue_drafts_version_check CHECK (version >= 1);
-- statement-break
CREATE INDEX idx_clue_drafts_case_review ON clue_drafts(case_id, review_status, updated_at);
