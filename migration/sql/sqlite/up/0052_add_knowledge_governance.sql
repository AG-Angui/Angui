CREATE TABLE knowledge_content_review_events (id TEXT PRIMARY KEY, knowledge_item_id TEXT NOT NULL, content_version INTEGER NOT NULL, event_type TEXT NOT NULL, actor_user_id TEXT NOT NULL, reason TEXT NOT NULL, created_at TEXT NOT NULL, FOREIGN KEY (knowledge_item_id) REFERENCES knowledge_items(id) ON DELETE CASCADE, FOREIGN KEY (actor_user_id) REFERENCES users(id), CHECK (event_type IN ('deidentified','reviewed','published','withdrawn')));
-- statement-break
CREATE INDEX idx_knowledge_content_review_events_item ON knowledge_content_review_events(knowledge_item_id, content_version, created_at);
-- statement-break
CREATE TABLE knowledge_attachments (id TEXT PRIMARY KEY, knowledge_item_id TEXT NOT NULL, file_name TEXT NOT NULL, storage_path TEXT NOT NULL UNIQUE, mime_type TEXT NOT NULL, byte_size INTEGER NOT NULL, created_at TEXT NOT NULL, FOREIGN KEY (knowledge_item_id) REFERENCES knowledge_items(id) ON DELETE CASCADE, CHECK (mime_type IN ('application/pdf')), CHECK (byte_size > 0));
-- statement-break
CREATE INDEX idx_knowledge_attachments_item ON knowledge_attachments(knowledge_item_id);
