ALTER TABLE voice_clue_candidates ADD COLUMN clue_draft_id VARCHAR(36) NULL REFERENCES clue_drafts(id) ON DELETE SET NULL;
-- statement-break
CREATE UNIQUE INDEX idx_voice_candidates_draft ON voice_clue_candidates(clue_draft_id);
