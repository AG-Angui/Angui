CREATE TABLE cases (
    id TEXT PRIMARY KEY NOT NULL,
    case_code TEXT NOT NULL UNIQUE,
    status TEXT NOT NULL,
    created_at TEXT NOT NULL,
    updated_at TEXT NOT NULL
);
-- statement-break
CREATE INDEX idx_cases_status ON cases(status);
