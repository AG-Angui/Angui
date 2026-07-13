CREATE TABLE cases (
    id VARCHAR(36) PRIMARY KEY,
    case_code VARCHAR(32) NOT NULL UNIQUE,
    status VARCHAR(32) NOT NULL,
    created_at VARCHAR(40) NOT NULL,
    updated_at VARCHAR(40) NOT NULL
) ENGINE=InnoDB;
-- statement-break
CREATE INDEX idx_cases_status ON cases(status);
