ALTER TABLE voice_clue_candidates DROP FOREIGN KEY fk_voice_candidate_draft;
-- statement-break
DROP INDEX idx_voice_candidates_draft ON voice_clue_candidates;
-- statement-break
ALTER TABLE voice_clue_candidates DROP COLUMN clue_draft_id;
