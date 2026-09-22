ALTER TABLE archive_drafts ADD COLUMN knowledge_item_id VARCHAR(191) NULL;
-- statement-break
CREATE INDEX idx_archive_drafts_knowledge_item ON archive_drafts(knowledge_item_id);
