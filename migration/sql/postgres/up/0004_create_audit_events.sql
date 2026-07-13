CREATE TABLE audit_events (
    id VARCHAR(36) PRIMARY KEY,
    case_id VARCHAR(36),
    actor VARCHAR(120) NOT NULL,
    action VARCHAR(64) NOT NULL,
    entity_type VARCHAR(64) NOT NULL,
    entity_id VARCHAR(36) NOT NULL,
    metadata_json TEXT,
    created_at VARCHAR(40) NOT NULL,
    CONSTRAINT fk_audit_events_case FOREIGN KEY (case_id) REFERENCES cases(id) ON DELETE SET NULL
);
-- statement-break
CREATE INDEX idx_audit_events_case_id ON audit_events(case_id);
-- statement-break
CREATE INDEX idx_audit_events_created_at ON audit_events(created_at);
