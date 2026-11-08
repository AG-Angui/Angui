ALTER TABLE knowledge_items ADD COLUMN external_id VARCHAR(191) NULL;
-- statement-break
CREATE INDEX idx_knowledge_items_external ON knowledge_items(knowledge_base_id, external_id);
-- statement-break
ALTER TABLE knowledge_terms ADD COLUMN parent_id VARCHAR(191) NULL;
-- statement-break
ALTER TABLE knowledge_terms ADD CONSTRAINT fk_knowledge_term_parent FOREIGN KEY (parent_id) REFERENCES knowledge_terms(id) ON DELETE RESTRICT;
-- statement-break
ALTER TABLE knowledge_terms ADD COLUMN description TEXT NOT NULL DEFAULT ('');
-- statement-break
ALTER TABLE knowledge_import_batches ADD COLUMN source_metadata_json JSON NOT NULL DEFAULT (JSON_OBJECT());
-- statement-break
CREATE INDEX idx_knowledge_terms_parent ON knowledge_terms(knowledge_base_id, parent_id);
