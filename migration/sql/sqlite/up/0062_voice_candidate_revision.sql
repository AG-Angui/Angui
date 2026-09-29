ALTER TABLE voice_clue_candidates ADD COLUMN returned_for_revision INTEGER NOT NULL DEFAULT 0 CHECK (returned_for_revision IN (0, 1));
