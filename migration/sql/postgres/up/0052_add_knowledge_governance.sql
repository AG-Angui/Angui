CREATE TABLE knowledge_content_review_events (id TEXT PRIMARY KEY, knowledge_item_id TEXT NOT NULL REFERENCES knowledge_items(id) ON DELETE CASCADE, content_version INTEGER NOT NULL, event_type TEXT NOT NULL CHECK (event_type IN ('deidentified','reviewed','published','withdrawn')), actor_user_id TEXT NOT NULL REFERENCES users(id), reason TEXT NOT NULL, created_at TEXT NOT NULL);
-- statement-break
CREATE INDEX idx_knowledge_content_review_events_item ON knowledge_content_review_events(knowledge_item_id, content_version, created_at);
-- statement-break
CREATE TABLE knowledge_attachments (id TEXT PRIMARY KEY, knowledge_item_id TEXT NOT NULL REFERENCES knowledge_items(id) ON DELETE CASCADE, file_name TEXT NOT NULL, storage_path TEXT NOT NULL UNIQUE, mime_type TEXT NOT NULL CHECK (mime_type IN ('application/pdf')), byte_size BIGINT NOT NULL CHECK (byte_size > 0), created_at TEXT NOT NULL);
-- statement-break
CREATE INDEX idx_knowledge_attachments_item ON knowledge_attachments(knowledge_item_id);
