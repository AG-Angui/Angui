CREATE TABLE clue_attributions (
    clue_id TEXT PRIMARY KEY,
    submitted_by_user_id TEXT,
    reviewed_by_user_id TEXT,
    reviewed_at TEXT,
    FOREIGN KEY (clue_id) REFERENCES clues(id) ON DELETE CASCADE,
    FOREIGN KEY (submitted_by_user_id) REFERENCES users(id) ON DELETE SET NULL,
    FOREIGN KEY (reviewed_by_user_id) REFERENCES users(id) ON DELETE SET NULL
);
-- statement-break
CREATE INDEX idx_clue_attributions_submitter ON clue_attributions(submitted_by_user_id);
