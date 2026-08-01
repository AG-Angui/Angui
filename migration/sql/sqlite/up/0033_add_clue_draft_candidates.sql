ALTER TABLE clue_drafts ADD COLUMN candidate_json TEXT NOT NULL DEFAULT '{}';
-- statement-break
ALTER TABLE clue_drafts ADD COLUMN review_status TEXT NOT NULL DEFAULT 'pending_review' CHECK (review_status IN ('pending_review', 'accepted', 'rejected'));
-- statement-break
ALTER TABLE clue_drafts ADD COLUMN reviewed_by_user_id TEXT;
-- statement-break
ALTER TABLE clue_drafts ADD COLUMN reviewed_at TEXT;
-- statement-break
ALTER TABLE clue_drafts ADD COLUMN review_reason TEXT;
-- statement-break
ALTER TABLE clue_drafts ADD COLUMN version INTEGER NOT NULL DEFAULT 1 CHECK (version >= 1);
-- statement-break
CREATE INDEX idx_clue_drafts_case_review ON clue_drafts(case_id, review_status, updated_at);
