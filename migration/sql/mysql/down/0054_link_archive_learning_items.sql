DROP INDEX idx_archive_drafts_knowledge_item ON archive_drafts;
-- statement-break
ALTER TABLE archive_drafts DROP COLUMN knowledge_item_id;
