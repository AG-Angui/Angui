CREATE TABLE clue_attributions (
    clue_id VARCHAR(36) PRIMARY KEY,
    submitted_by_user_id VARCHAR(36),
    reviewed_by_user_id VARCHAR(36),
    reviewed_at VARCHAR(40),
    CONSTRAINT fk_clue_attributions_clue FOREIGN KEY (clue_id) REFERENCES clues(id) ON DELETE CASCADE,
    CONSTRAINT fk_clue_attributions_submitter FOREIGN KEY (submitted_by_user_id) REFERENCES users(id) ON DELETE SET NULL,
    CONSTRAINT fk_clue_attributions_reviewer FOREIGN KEY (reviewed_by_user_id) REFERENCES users(id) ON DELETE SET NULL
) ENGINE=InnoDB;
-- statement-break
CREATE INDEX idx_clue_attributions_submitter ON clue_attributions(submitted_by_user_id);
