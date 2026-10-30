ALTER TABLE voice_clue_candidates ADD COLUMN clue_draft_id VARCHAR(36) NULL;
-- statement-break
CREATE UNIQUE INDEX idx_voice_candidates_draft ON voice_clue_candidates(clue_draft_id);
-- statement-break
ALTER TABLE voice_clue_candidates ADD CONSTRAINT fk_voice_candidate_draft FOREIGN KEY (clue_draft_id) REFERENCES clue_drafts(id) ON DELETE SET NULL;
