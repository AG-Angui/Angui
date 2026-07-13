CREATE TABLE clues (
    id VARCHAR(36) PRIMARY KEY,
    case_id VARCHAR(36) NOT NULL,
    status VARCHAR(32) NOT NULL,
    source VARCHAR(64) NOT NULL,
    content TEXT NOT NULL,
    occurred_at VARCHAR(40),
    location_text VARCHAR(500),
    created_at VARCHAR(40) NOT NULL,
    updated_at VARCHAR(40) NOT NULL,
    CONSTRAINT fk_clues_case FOREIGN KEY (case_id) REFERENCES cases(id) ON DELETE CASCADE
) ENGINE=InnoDB;
-- statement-break
CREATE INDEX idx_clues_case_id ON clues(case_id);
-- statement-break
CREATE INDEX idx_clues_status ON clues(status);
