CREATE TABLE audit_events (
    id TEXT PRIMARY KEY NOT NULL,
    case_id TEXT,
    actor TEXT NOT NULL,
    action TEXT NOT NULL,
    entity_type TEXT NOT NULL,
    entity_id TEXT NOT NULL,
    metadata_json TEXT,
    created_at TEXT NOT NULL,
    FOREIGN KEY (case_id) REFERENCES cases(id) ON DELETE SET NULL
);
-- statement-break
CREATE INDEX idx_audit_events_case_id ON audit_events(case_id);
-- statement-break
CREATE INDEX idx_audit_events_created_at ON audit_events(created_at);
