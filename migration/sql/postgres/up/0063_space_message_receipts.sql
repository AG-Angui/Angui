CREATE TABLE space_message_receipts (id TEXT PRIMARY KEY, space_id TEXT NOT NULL REFERENCES collaboration_spaces(id) ON DELETE CASCADE, message_id TEXT NOT NULL REFERENCES space_messages(id) ON DELETE CASCADE, user_id TEXT NOT NULL REFERENCES users(id) ON DELETE CASCADE, status TEXT NOT NULL CHECK (status IN ('delivered', 'acknowledged')), acknowledged_at TEXT NOT NULL, UNIQUE(space_id, message_id, user_id));
-- statement-break
CREATE INDEX idx_space_message_receipts_message ON space_message_receipts(space_id, message_id);
