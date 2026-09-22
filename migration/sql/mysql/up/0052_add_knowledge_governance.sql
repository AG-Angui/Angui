CREATE TABLE knowledge_content_review_events (id VARCHAR(191) PRIMARY KEY, knowledge_item_id VARCHAR(191) NOT NULL, content_version INTEGER NOT NULL, event_type VARCHAR(32) NOT NULL, actor_user_id VARCHAR(191) NOT NULL, reason TEXT NOT NULL, created_at VARCHAR(64) NOT NULL, CONSTRAINT fk_knowledge_review_item FOREIGN KEY (knowledge_item_id) REFERENCES knowledge_items(id) ON DELETE CASCADE, CONSTRAINT fk_knowledge_review_actor FOREIGN KEY (actor_user_id) REFERENCES users(id), CONSTRAINT chk_knowledge_review_event CHECK (event_type IN ('deidentified','reviewed','published','withdrawn')));
-- statement-break
CREATE INDEX idx_knowledge_content_review_events_item ON knowledge_content_review_events(knowledge_item_id, content_version, created_at);
-- statement-break
CREATE TABLE knowledge_attachments (id VARCHAR(191) PRIMARY KEY, knowledge_item_id VARCHAR(191) NOT NULL, file_name VARCHAR(255) NOT NULL, storage_path VARCHAR(512) NOT NULL UNIQUE, mime_type VARCHAR(128) NOT NULL, byte_size BIGINT NOT NULL, CONSTRAINT fk_knowledge_attachment_item FOREIGN KEY (knowledge_item_id) REFERENCES knowledge_items(id) ON DELETE CASCADE, CONSTRAINT chk_knowledge_attachment_mime CHECK (mime_type IN ('application/pdf')), CONSTRAINT chk_knowledge_attachment_size CHECK (byte_size > 0));
-- statement-break
CREATE INDEX idx_knowledge_attachments_item ON knowledge_attachments(knowledge_item_id);
