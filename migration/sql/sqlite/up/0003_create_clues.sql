CREATE TABLE clues (
    id TEXT PRIMARY KEY NOT NULL,
    case_id TEXT NOT NULL,
    status TEXT NOT NULL,
    source TEXT NOT NULL,
    content TEXT NOT NULL,
    occurred_at TEXT,
    location_text TEXT,
    created_at TEXT NOT NULL,
    updated_at TEXT NOT NULL,
    FOREIGN KEY (case_id) REFERENCES cases(id) ON DELETE CASCADE
);
-- statement-break
CREATE INDEX idx_clues_case_id ON clues(case_id);
-- statement-break
CREATE INDEX idx_clues_status ON clues(status);
