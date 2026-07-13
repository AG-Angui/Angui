DROP INDEX idx_audit_events_created_at ON audit_events;
-- statement-break
DROP INDEX idx_audit_events_case_id ON audit_events;
-- statement-break
DROP TABLE IF EXISTS audit_events;
